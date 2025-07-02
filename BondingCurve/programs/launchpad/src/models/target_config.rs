use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct TargetConfig {
    pub token_target_amount: u64,
    pub token_mint: Pubkey,
}

impl TargetConfig {
    pub const CONFIG_PREFIX: &'static [u8; 6] = b"config";
}
