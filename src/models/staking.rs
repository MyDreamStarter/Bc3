use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StakingPool {
    pub to_airdrop: u64,
    pub padding: [u8; 32],
}

impl StakingPool {
    pub const SIGNER_PDA_PREFIX: &'static [u8; 6] = b"signer";
}
