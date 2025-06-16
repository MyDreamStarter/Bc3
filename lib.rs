use anchor_lang::prelude::*;

declare_id!("GaKH1997A2Zai7T6s1NuWKzjVEvM4mFmsaBz3XeKD3Z9");

mod consts;
mod err;
mod libraries;
mod math;
mod models;

use core as core_;

#[program]
pub mod launchpad {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
