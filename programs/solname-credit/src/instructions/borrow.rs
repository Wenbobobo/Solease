use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::state::{LoanAccount, LoanStatus, LoanType, PoolState, OfferAccount};
use crate::constants::{LOAN_SEED, POOL_SEED, OFFER_SEED, VAULT_SEED};
use crate::error::ErrorCode;

const NAME_SERVICE_ID: Pubkey = pubkey!("namesLPneVptA9Z5rqUDD9tMTWEJwofgaYwp8cawRkX");

#[derive(Accounts)]
#[instruction(mode: LoanTypeInput, offer_id: Option<Pubkey>)]
pub struct SetupCollateral<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,

    /// CHECK: Validated manualy via data inspection
    #[account(mut)]
    pub domain_registry: AccountInfo<'info>,

    #[account(
        init,
        seeds = [LOAN_SEED, domain_registry.key().as_ref()],
        bump,
        payer = borrower,
        space = 8 + std::mem::size_of::<LoanAccount>()
    )]
    pub loan_account: Account<'info, LoanAccount>,

    /// CHECK: This PDA will become the new owner of the domain
    #[account(seeds = [b"escrow", loan_account.key().as_ref()], bump)]
    pub escrow_pda: AccountInfo<'info>,

    /// CHECK: Name Service Program
    #[account(address = NAME_SERVICE_ID)]
    pub name_service_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoanTypeInput {
    Pool,
    P2P,
}

pub fn setup_collateral(
    ctx: Context<SetupCollateral>,
    mode: LoanTypeInput,
    offer_id: Option<Pubkey>
) -> Result<()> {
    let loan_account = &mut ctx.accounts.loan_account;

    // Check Owner
    let data = ctx.accounts.domain_registry.try_borrow_data()?;
    if data.len() < 96 {
        return err!(ErrorCode::InvalidDomainOwner);
    }
    let owner_slice = &data[32..64];
    let owner = Pubkey::try_from(owner_slice).unwrap();
    require_keys_eq!(owner, ctx.accounts.borrower.key(), ErrorCode::InvalidDomainOwner);

    // CPI Transfer to Escrow
    let transfer_instruction_data = vec![1]; // Tag for Transfer
    let transfer_accounts = vec![
        AccountMeta::new(ctx.accounts.domain_registry.key(), false),
        AccountMeta::new(ctx.accounts.escrow_pda.key(), false),
        AccountMeta::new_readonly(ctx.accounts.borrower.key(), true),
    ];
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.name_service_program.key(),
        accounts: transfer_accounts,
        data: [transfer_instruction_data, ctx.accounts.escrow_pda.key().to_bytes().to_vec()].concat(),
    };
    anchor_lang::solana_program::program::invoke(
        &ix,
        &[
            ctx.accounts.domain_registry.to_account_info(),
            ctx.accounts.escrow_pda.to_account_info(),
            ctx.accounts.borrower.to_account_info(),
        ],
    )?;

    loan_account.borrower = ctx.accounts.borrower.key();
    loan_account.domain_registry = ctx.accounts.domain_registry.key();
    loan_account.escrow_pda = ctx.accounts.escrow_pda.key();
    loan_account.status = LoanStatus::SetupPending;
    loan_account.bump = ctx.bumps.loan_account;

    match mode {
        LoanTypeInput::Pool => {
            loan_account.loan_type = LoanType::Pool;
        },
        LoanTypeInput::P2P => {
            loan_account.loan_type = LoanType::P2P;
            if let Some(offer) = offer_id {
                loan_account.lender_source = offer;
            } else {
                return err!(ErrorCode::Unauthorized);
            }
        }
    }

    Ok(())
}

#[derive(Accounts)]
pub struct VerifyAndWithdrawPool<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(
        mut,
        seeds = [LOAN_SEED, loan_account.domain_registry.as_ref()],
        bump = loan_account.bump,
        constraint = loan_account.borrower == borrower.key(),
        constraint = loan_account.status == LoanStatus::SetupPending,
        constraint = loan_account.loan_type == LoanType::Pool
    )]
    pub loan_account: Account<'info, LoanAccount>,

    #[account(mut)]
    pub pool_state: Account<'info, PoolState>,

    /// CHECK: Validated seeds
    #[account(seeds = [VAULT_SEED, pool_state.key().as_ref()], bump)]
    pub vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, pool_state.key().as_ref(), b"token"],
        bump,
        token::authority = vault_authority
    )]
    pub pool_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub borrower_usdc: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn verify_and_withdraw_pool(ctx: Context<VerifyAndWithdrawPool>) -> Result<()> {
    let loan = &mut ctx.accounts.loan_account;
    let pool = &mut ctx.accounts.pool_state;
    let now = ctx.accounts.clock.unix_timestamp;

    // MVP Policy: Fixed small loan for Pool
    let principal = 10_000_000; // 10 USDC
    let duration = 86400 * 14; // 14 days
    let apr = 1000; // 10%

    // Update Loan
    loan.principal_amount = principal;
    loan.apr_bps = apr;
    loan.start_ts = now;
    loan.due_ts = now + duration;
    loan.status = LoanStatus::Active;
    loan.lender_source = pool.key();

    // Transfer Funds
    let pool_key = pool.key();
    let seeds = &[
        VAULT_SEED,
        pool_key.as_ref(),
        &[ctx.bumps.vault_authority]
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.pool_vault.to_account_info(),
        to: ctx.accounts.borrower_usdc.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer);
    token::transfer(cpi_ctx, principal)?;

    Ok(())
}

#[derive(Accounts)]
pub struct VerifyAndWithdrawP2P<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(
        mut,
        seeds = [LOAN_SEED, loan_account.domain_registry.as_ref()],
        bump = loan_account.bump,
        constraint = loan_account.borrower == borrower.key(),
        constraint = loan_account.status == LoanStatus::SetupPending,
        constraint = loan_account.loan_type == LoanType::P2P,
        constraint = loan_account.lender_source == offer_account.key()
    )]
    pub loan_account: Account<'info, LoanAccount>,

    #[account(mut)]
    pub offer_account: Account<'info, OfferAccount>,

    #[account(
        mut,
        seeds = [VAULT_SEED, offer_account.key().as_ref()],
        bump,
        token::authority = offer_account
    )]
    pub offer_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub borrower_usdc: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn verify_and_withdraw_p2p(ctx: Context<VerifyAndWithdrawP2P>) -> Result<()> {
    let now = ctx.accounts.clock.unix_timestamp;

    // We access fields before borrowing offer_account mutably via ctx.accounts if possible,
    // but here we need mutable access for update.
    // To avoid immutable borrow error later when getting account_info, we extract needed values first.

    let principal = ctx.accounts.offer_account.principal;
    let apr_bps = ctx.accounts.offer_account.apr_bps;
    let duration = ctx.accounts.offer_account.duration_seconds;
    let expiry = ctx.accounts.offer_account.offer_expiry;
    let is_active = ctx.accounts.offer_account.is_active;
    let lender = ctx.accounts.offer_account.lender;
    let nonce = ctx.accounts.offer_account.nonce;
    let bump = ctx.accounts.offer_account.bump;

    require!(is_active, ErrorCode::OfferExpired);
    require!(now < expiry, ErrorCode::OfferExpired);

    // Update Loan
    let loan = &mut ctx.accounts.loan_account;
    loan.principal_amount = principal;
    loan.apr_bps = apr_bps;
    loan.start_ts = now;
    loan.due_ts = now + duration;
    loan.status = LoanStatus::Active;

    // Close Offer (Mark taken)
    let offer_account = &mut ctx.accounts.offer_account;
    offer_account.is_active = false;

    // Transfer Funds
    // We need the OfferAccount PDA to sign.
    // The seeds depend on the lender and nonce.
    let nonce_bytes = nonce.to_le_bytes();
    let seeds = &[
        OFFER_SEED,
        lender.as_ref(),
        nonce_bytes.as_ref(),
        &[bump]
    ];
    let signer = &[&seeds[..]];

    // Use offer_account.to_account_info() directly.
    let cpi_accounts = Transfer {
        from: ctx.accounts.offer_vault.to_account_info(),
        to: ctx.accounts.borrower_usdc.to_account_info(),
        authority: offer_account.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer);
    token::transfer(cpi_ctx, principal)?;

    Ok(())
}

#[derive(Accounts)]
pub struct Repay<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(
        mut,
        seeds = [LOAN_SEED, loan_account.domain_registry.as_ref()],
        bump = loan_account.bump,
        constraint = loan_account.borrower == borrower.key(),
        constraint = loan_account.status == LoanStatus::Active
    )]
    pub loan_account: Account<'info, LoanAccount>,

    #[account(mut)]
    pub borrower_usdc: Account<'info, TokenAccount>,

    #[account(mut)]
    pub destination_vault: Account<'info, TokenAccount>, // Pool or Lender

    /// CHECK: PDA owning domain
    #[account(seeds = [b"escrow", loan_account.key().as_ref()], bump)]
    pub escrow_pda: AccountInfo<'info>,

    /// CHECK: Original domain registry
    #[account(mut)]
    pub domain_registry: AccountInfo<'info>,

    /// CHECK: Name Service
    #[account(address = NAME_SERVICE_ID)]
    pub name_service_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn repay(ctx: Context<Repay>) -> Result<()> {
    let loan = &mut ctx.accounts.loan_account;

    // Calculate Repayment
    let amount_due = loan.principal_amount;

    // Transfer USDC
    let cpi_accounts = Transfer {
        from: ctx.accounts.borrower_usdc.to_account_info(),
        to: ctx.accounts.destination_vault.to_account_info(),
        authority: ctx.accounts.borrower.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount_due)?;

    // Return Domain
    let loan_key = loan.key();
    let seeds = &[
        b"escrow",
        loan_key.as_ref(),
        &[ctx.bumps.escrow_pda]
    ];
    let signer = &[&seeds[..]];

    let transfer_instruction_data = vec![1]; // Tag for Transfer
    let transfer_accounts = vec![
        AccountMeta::new(ctx.accounts.domain_registry.key(), false),
        AccountMeta::new(ctx.accounts.borrower.key(), false), // New owner = borrower
        AccountMeta::new_readonly(ctx.accounts.escrow_pda.key(), true), // Signer
    ];
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ctx.accounts.name_service_program.key(),
        accounts: transfer_accounts,
        data: [transfer_instruction_data, ctx.accounts.borrower.key().to_bytes().to_vec()].concat(),
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.domain_registry.to_account_info(),
            ctx.accounts.borrower.to_account_info(),
            ctx.accounts.escrow_pda.to_account_info(),
        ],
        signer
    )?;

    loan.status = LoanStatus::Repaid;
    loan.repaid_amount = amount_due;

    Ok(())
}
