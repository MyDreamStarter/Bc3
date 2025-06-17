mod consts;
mod endpoints;
mod err;
mod libraries;
mod math;
mod models;

use crate::endpoints::*;
use anchor_lang::prelude::*;
use core as core_;

declare_id!("GaKH1997A2Zai7T6s1NuWKzjVEvM4mFmsaBz3XeKD3Z9");

/// # GAMING TERMINAL Launchpad Program
///
/// A comprehensive memecoin launchpad protocol on Solana featuring:
/// - Bonding curve token launches
/// - Automated market maker integration (Meteora)
/// - Vesting and staking mechanisms
/// - Points-based rewards system
/// - Fair launch mechanics with anti-rug features
#[program]
pub mod launchpad {
    use super::*;

    // ===== Pool Creation & Management =====

    /// Creates a new bonding curve pool for a memecoin launch
    ///
    /// # Arguments
    /// * `airdropped_tokens` - Amount of tokens reserved for airdrops (max 100M)
    /// * `vesting_period` - Lock period for tokens in seconds
    pub fn new_pool(
        ctx: Context<NewPool>,
        airdropped_tokens: u64,
        vesting_period: u64,
    ) -> Result<()> {
        new_pool::handle(ctx, airdropped_tokens, vesting_period as i64)
    }

    /// Creates token metadata for the launched memecoin
    ///
    /// # Arguments
    /// * `name` - Token name (e.g., "Doge Coin")
    /// * `symbol` - Token symbol (e.g., "DOGE")
    /// * `uri` - Metadata URI pointing to off-chain JSON
    pub fn create_metadata(
        ctx: Context<CreateMetadata>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        create_metadata::handle(ctx, name, symbol, uri)
    }

    // ===== Trading Functions =====

    /// Preview swap: selling meme tokens for SOL
    /// Returns expected amounts without executing trade
    pub fn get_swap_x_amt(
        ctx: Context<GetSwapXAmt>,
        coin_in_amount: u64,
        coin_y_min_value: u64,
    ) -> Result<()> {
        get_swap_x_amt::handle(ctx, coin_in_amount, coin_y_min_value)
    }

    /// Execute swap: sell meme tokens for SOL
    /// Uses direct token transfer (no ticket system)
    pub fn swap_x(
        ctx: Context<SwapCoinX>,
        coin_in_amount: u64,
        coin_y_min_value: u64,
    ) -> Result<()> {
        swap_x::handle(ctx, coin_in_amount, coin_y_min_value)
    }

    /// Preview swap: buying meme tokens with SOL
    /// Returns expected amounts without executing trade
    pub fn get_swap_y_amt(
        ctx: Context<GetSwapYAmt>,
        coin_in_amount: u64,
        coin_x_min_value: u64,
    ) -> Result<()> {
        get_swap_y_amt::handle(ctx, coin_in_amount, coin_x_min_value)
    }

    /// Execute swap: buy meme tokens with SOL
    /// Direct transfer to user's wallet + points rewards
    pub fn swap_y(
        ctx: Context<SwapCoinY>,
        coin_in_amount: u64,
        coin_x_min_value: u64,
    ) -> Result<()> {
        swap_y::handle(ctx, coin_in_amount, coin_x_min_value)
    }

    /// Send airdrop funds to designated recipient
    pub fn send_airdrop_funds(ctx: Context<SendAirdropFunds>) -> Result<()> {
        send_airdrop_funds::handle(ctx)
    }
}
