use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct PointsEpoch {
    pub epoch_number: u64,
    pub points_per_sol_num: u64,
    pub points_per_sol_denom: u64,
    pub padding: [u8; 8],
}
