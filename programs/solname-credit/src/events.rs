use anchor_lang::prelude::*;

#[event]
pub struct GlobalInitialized {
    pub admin: Pubkey,
}

#[event]
pub struct PoolInitialized {
    pub mint: Pubkey,
}
