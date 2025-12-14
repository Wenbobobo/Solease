use anchor_lang::prelude::*;

pub mod state;
pub mod instructions;
pub mod error;
pub mod events;
pub mod constants;

use instructions::admin::*;
use instructions::lp::*;
use instructions::p2p::*;
use instructions::borrow::*;
use instructions::liquidation::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod solname_credit {
    use super::*;

    pub fn init_global(ctx: Context<InitGlobal>, params: GlobalParams) -> Result<()> {
        instructions::admin::init_global(ctx, params)
    }

    pub fn init_pool(ctx: Context<InitPool>) -> Result<()> {
        instructions::admin::init_pool(ctx)
    }

    // pub fn deposit_liquidity(ctx: Context<DepositLiquidity>, amount: u64) -> Result<()> {
    //     instructions::lp::deposit_liquidity(ctx, amount)
    // }

    pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, shares: u64) -> Result<()> {
        instructions::lp::withdraw_liquidity(ctx, shares)
    }

    pub fn create_offer(
        ctx: Context<CreateOffer>,
        nonce: u64,
        principal: u64,
        apr_bps: u16,
        duration_seconds: i64,
        offer_expiry: i64
    ) -> Result<()> {
        instructions::p2p::create_offer(ctx, nonce, principal, apr_bps, duration_seconds, offer_expiry)
    }

    pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
        instructions::p2p::cancel_offer(ctx)
    }

    pub fn setup_collateral(
        ctx: Context<SetupCollateral>,
        mode: LoanTypeInput,
        offer_id: Option<Pubkey>
    ) -> Result<()> {
        instructions::borrow::setup_collateral(ctx, mode, offer_id)
    }

    pub fn verify_and_withdraw_pool(ctx: Context<VerifyAndWithdrawPool>) -> Result<()> {
        instructions::borrow::verify_and_withdraw_pool(ctx)
    }

    pub fn verify_and_withdraw_p2p(ctx: Context<VerifyAndWithdrawP2P>) -> Result<()> {
        instructions::borrow::verify_and_withdraw_p2p(ctx)
    }

    pub fn repay(ctx: Context<Repay>) -> Result<()> {
        instructions::borrow::repay(ctx)
    }

    pub fn enter_grace(ctx: Context<EnterGrace>) -> Result<()> {
        instructions::liquidation::enter_grace(ctx)
    }

    pub fn start_auction(ctx: Context<StartAuction>) -> Result<()> {
        instructions::liquidation::start_auction(ctx)
    }

    pub fn place_bid(ctx: Context<PlaceBid>, amount: u64) -> Result<()> {
        instructions::liquidation::place_bid(ctx, amount)
    }

    pub fn buy_it_now(ctx: Context<BuyItNow>) -> Result<()> {
        instructions::liquidation::buy_it_now(ctx)
    }

    pub fn settle_auction(ctx: Context<SettleAuction>) -> Result<()> {
        instructions::liquidation::settle_auction(ctx)
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct GlobalParams {
    pub global_cap: u64,
    pub grace_period_seconds: i64,
    pub min_bid_increment_bps: u16,
    pub auction_duration_seconds: i64,
}
