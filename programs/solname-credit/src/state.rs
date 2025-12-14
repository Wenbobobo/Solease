use anchor_lang::prelude::*;

#[account]
pub struct GlobalState {
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
    pub global_cap: u64,
    pub grace_period_seconds: i64,
    pub min_bid_increment_bps: u16,
    pub auction_duration_seconds: i64,
}

#[account]
pub struct PoolState {
    pub mint: Pubkey,
    pub vault_authority: Pubkey,
    pub total_shares: u64,
    pub total_assets: u64,
    pub bump: u8,
}

#[account]
pub struct LpPosition {
    pub owner: Pubkey,
    pub shares: u64,
    pub bump: u8,
}

#[account]
pub struct LoanAccount {
    pub borrower: Pubkey,
    pub domain_registry: Pubkey,
    pub escrow_pda: Pubkey,
    pub principal_amount: u64,
    pub repaid_amount: u64,
    pub apr_bps: u16,
    pub start_ts: i64,
    pub due_ts: i64,
    pub grace_end_ts: i64,
    pub last_update_ts: i64,
    pub status: LoanStatus,
    pub loan_type: LoanType,
    pub lender_source: Pubkey,
    pub record_payout: Pubkey,
    pub bump: u8,
}

#[account]
pub struct OfferAccount {
    pub lender: Pubkey,
    pub principal: u64,
    pub apr_bps: u16,
    pub duration_seconds: i64,
    pub offer_expiry: i64,
    pub is_active: bool,
    pub nonce: u64,
    pub bump: u8,
}

#[account]
pub struct AuctionAccount {
    pub loan: Pubkey,
    pub start_ts: i64,
    pub end_ts: i64,
    pub highest_bid: u64,
    pub highest_bidder: Pubkey,
    pub start_price: u64,
    pub end_price: u64,
    pub min_bid: u64,
    pub status: AuctionStatus,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoanStatus {
    SetupPending,
    Active,
    Grace,
    AuctionLive,
    Repaid,
    Defaulted,
    Settled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoanType {
    Pool,
    P2P,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AuctionStatus {
    Live,
    Ended,
}
