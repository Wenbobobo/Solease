use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::{GlobalState, PoolState};
use crate::constants::{GLOBAL_SEED, POOL_SEED, VAULT_SEED};
use crate::GlobalParams;

#[derive(Accounts)]
pub struct InitGlobal<'info> {
    #[account(init, seeds = [GLOBAL_SEED], bump, payer = admin, space = 8 + std::mem::size_of::<GlobalState>())]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
}

pub fn init_global(ctx: Context<InitGlobal>, params: GlobalParams) -> Result<()> {
    let global_state = &mut ctx.accounts.global_state;
    global_state.admin = ctx.accounts.admin.key();
    global_state.usdc_mint = ctx.accounts.usdc_mint.key();
    global_state.global_cap = params.global_cap;
    global_state.grace_period_seconds = params.grace_period_seconds;
    global_state.min_bid_increment_bps = params.min_bid_increment_bps;
    global_state.auction_duration_seconds = params.auction_duration_seconds;
    Ok(())
}

#[derive(Accounts)]
pub struct InitPool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [GLOBAL_SEED], bump, has_one = admin)]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        init,
        seeds = [POOL_SEED, global_state.usdc_mint.as_ref()],
        bump,
        payer = admin,
        space = 8 + std::mem::size_of::<PoolState>()
    )]
    pub pool_state: Account<'info, PoolState>,

    /// CHECK: This is a PDA used as vault authority
    #[account(seeds = [VAULT_SEED, pool_state.key().as_ref()], bump)]
    pub vault_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = admin,
        token::mint = usdc_mint,
        token::authority = vault_authority,
        seeds = [VAULT_SEED, pool_state.key().as_ref(), b"token"],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(address = global_state.usdc_mint)]
    pub usdc_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn init_pool(ctx: Context<InitPool>) -> Result<()> {
    let pool_state = &mut ctx.accounts.pool_state;
    pool_state.vault_authority = ctx.accounts.vault_authority.key();
    pool_state.total_shares = 0;
    pool_state.total_assets = 0;
    pool_state.bump = ctx.bumps.pool_state;
    Ok(())
}
