use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{GlobalState, PoolState, LpPosition};
use crate::constants::{GLOBAL_SEED, POOL_SEED, VAULT_SEED};
use crate::error::ErrorCode;

/*
#[derive(Accounts)]
pub struct DepositLiquidity<'info> {
    #[account(mut)]
    pub liquidity_provider: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [POOL_SEED, global_state.usdc_mint.as_ref()],
        bump
    )]
    pub pool_state: Account<'info, PoolState>,

    /// CHECK: This is a PDA used as vault authority
    #[account(seeds = [VAULT_SEED, pool_state.key().as_ref()], bump)]
    pub vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, pool_state.key().as_ref(), b"token"],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        // constraint = user_usdc.mint == global_state.usdc_mint
    )]
    pub user_usdc: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = liquidity_provider,
        seeds = [b"lp", pool_state.key().as_ref(), liquidity_provider.key().as_ref()],
        bump,
        space = 8 + 32 + 8 + 1
    )]
    pub lp_position: Account<'info, LpPosition>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn deposit_liquidity(ctx: Context<DepositLiquidity>, amount: u64) -> Result<()> {
    // ... code ...
    Ok(())
}
*/

#[derive(Accounts)]
pub struct WithdrawLiquidity<'info> {
    #[account(mut)]
    pub liquidity_provider: Signer<'info>,

    #[account(seeds = [GLOBAL_SEED], bump)]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [POOL_SEED, global_state.usdc_mint.as_ref()],
        bump
    )]
    pub pool_state: Account<'info, PoolState>,

    /// CHECK: PDA authority
    #[account(seeds = [VAULT_SEED, pool_state.key().as_ref()], bump)]
    pub vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, pool_state.key().as_ref(), b"token"],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_usdc.mint == global_state.usdc_mint
    )]
    pub user_usdc: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"lp", pool_state.key().as_ref(), liquidity_provider.key().as_ref()],
        bump = lp_position.bump,
        constraint = lp_position.owner == liquidity_provider.key()
    )]
    pub lp_position: Account<'info, LpPosition>,

    pub token_program: Program<'info, Token>,
}

pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, shares_to_burn: u64) -> Result<()> {
    let pool_state = &mut ctx.accounts.pool_state;
    let lp_position = &mut ctx.accounts.lp_position;

    require!(lp_position.shares >= shares_to_burn, ErrorCode::InsufficientLiquidity);

    // Calculate amount to return
    let amount_to_return = (shares_to_burn as u128)
        .checked_mul(pool_state.total_assets as u128)
        .unwrap()
        .checked_div(pool_state.total_shares as u128)
        .unwrap() as u64;

    require!(ctx.accounts.vault.amount >= amount_to_return, ErrorCode::InsufficientLiquidity);

    // Update state
    lp_position.shares = lp_position.shares.checked_sub(shares_to_burn).unwrap();
    pool_state.total_shares = pool_state.total_shares.checked_sub(shares_to_burn).unwrap();
    pool_state.total_assets = pool_state.total_assets.checked_sub(amount_to_return).unwrap();

    // Transfer USDC from vault to user
    let pool_key = pool_state.key();
    let seeds = &[
        VAULT_SEED,
        pool_key.as_ref(),
        &[ctx.bumps.vault_authority]
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.user_usdc.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, amount_to_return)?;

    Ok(())
}
