use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, CloseAccount};
use crate::state::OfferAccount;
use crate::constants::{OFFER_SEED, VAULT_SEED};
use crate::error::ErrorCode;

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct CreateOffer<'info> {
    #[account(mut)]
    pub lender: Signer<'info>,

    #[account(
        init,
        seeds = [OFFER_SEED, lender.key().as_ref(), nonce.to_le_bytes().as_ref()],
        bump,
        payer = lender,
        space = 8 + std::mem::size_of::<OfferAccount>()
    )]
    pub offer_account: Account<'info, OfferAccount>,

    #[account(
        init,
        seeds = [VAULT_SEED, offer_account.key().as_ref()],
        bump,
        payer = lender,
        token::mint = usdc_mint,
        token::authority = offer_account
    )]
    pub offer_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub lender_usdc: Account<'info, TokenAccount>,

    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_offer(
    ctx: Context<CreateOffer>,
    nonce: u64,
    principal: u64,
    apr_bps: u16,
    duration_seconds: i64,
    offer_expiry: i64
) -> Result<()> {
    let offer_account = &mut ctx.accounts.offer_account;
    offer_account.lender = ctx.accounts.lender.key();
    offer_account.principal = principal;
    offer_account.apr_bps = apr_bps;
    offer_account.duration_seconds = duration_seconds;
    offer_account.offer_expiry = offer_expiry;
    offer_account.is_active = true;
    offer_account.nonce = nonce;
    offer_account.bump = ctx.bumps.offer_account;

    // Transfer principal to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.lender_usdc.to_account_info(),
        to: ctx.accounts.offer_vault.to_account_info(),
        authority: ctx.accounts.lender.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, principal)?;

    Ok(())
}

#[derive(Accounts)]
pub struct CancelOffer<'info> {
    #[account(mut)]
    pub lender: Signer<'info>,

    #[account(
        mut,
        has_one = lender,
        close = lender
    )]
    pub offer_account: Account<'info, OfferAccount>,

    #[account(
        mut,
        seeds = [VAULT_SEED, offer_account.key().as_ref()],
        bump,
        token::authority = offer_account
    )]
    pub offer_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub lender_usdc: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
    let offer_account = &ctx.accounts.offer_account;

    // Transfer funds back to lender
    let nonce_bytes = offer_account.nonce.to_le_bytes();
    let seeds = &[
        OFFER_SEED,
        offer_account.lender.as_ref(),
        nonce_bytes.as_ref(),
        &[offer_account.bump]
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.offer_vault.to_account_info(),
        to: ctx.accounts.lender_usdc.to_account_info(),
        authority: ctx.accounts.offer_account.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, ctx.accounts.offer_vault.amount)?;

    // Close the vault
    let close_accounts = CloseAccount {
        account: ctx.accounts.offer_vault.to_account_info(),
        destination: ctx.accounts.lender.to_account_info(),
        authority: ctx.accounts.offer_account.to_account_info(),
    };
    let close_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), close_accounts, signer);
    token::close_account(close_ctx)?;

    Ok(())
}
