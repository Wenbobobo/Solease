use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Global state already initialized.")]
    GlobalAlreadyInitialized,
    #[msg("Pool already initialized.")]
    PoolAlreadyInitialized,
    #[msg("Unauthorized access.")]
    Unauthorized,
    #[msg("Math overflow.")]
    MathOverflow,
    #[msg("Insufficient liquidity in pool.")]
    InsufficientLiquidity,
    #[msg("Invalid domain owner.")]
    InvalidDomainOwner,
    #[msg("Domain is tokenized.")]
    DomainIsTokenized,
    #[msg("Loan already active.")]
    LoanAlreadyActive,
    #[msg("Loan not ready for withdrawal.")]
    LoanNotSetup,
    #[msg("Loan not due.")]
    LoanNotDue,
    #[msg("Offer expired.")]
    OfferExpired,
    #[msg("Bid too low.")]
    BidTooLow,
    #[msg("Auction ended.")]
    AuctionEnded,
}
