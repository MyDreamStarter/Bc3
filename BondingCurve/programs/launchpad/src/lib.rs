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
/// - Bonding curve token launches with fair price discovery
/// - Automated market maker integration (Raydium CPMM)
/// - Automatic migration at 80% threshold for deeper liquidity  
/// - Vesting and staking mechanisms for anti-rug protection
/// - Points-based rewards system with referral bonuses
/// - Fair launch mechanics with built-in safeguards
#[program]
pub mod launchpad {
    use super::*;

    // ===== Pool Creation & Management =====

    /// Creates a new bonding curve pool for a memecoin launch
    ///
    /// # Arguments
    /// * `airdropped_tokens` - Amount of tokens reserved for airdrops (max 100M)
    pub fn new_pool(ctx: Context<NewPool>, airdropped_tokens: u64) -> Result<()> {
        new_pool::handle(ctx, airdropped_tokens)
    }

    /// Creates token metadata for the launched memecoin
    ///
    /// # Arguments
    /// * `name` - Token name (e.g., "Doge Coin")
    /// * `symbol` - Token symbol (e.g., "DOGE")  
    /// * `uri` - Metadata URI pointing to off-chain JSON with image/description
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
    ///
    /// # Arguments
    /// * `coin_in_amount` - Amount of meme tokens to sell
    /// * `coin_y_min_value` - Minimum SOL to receive (slippage protection)
    pub fn get_swap_x_amt(
        ctx: Context<GetSwapXAmt>,
        coin_in_amount: u64,
        coin_y_min_value: u64,
    ) -> Result<()> {
        get_swap_x_amt::handle(ctx, coin_in_amount, coin_y_min_value)
    }

    /// Execute swap: sell meme tokens for SOL
    /// Uses direct token transfer with bonding curve pricing
    ///
    /// # Arguments
    /// * `coin_in_amount` - Amount of meme tokens to sell
    /// * `coin_y_min_value` - Minimum SOL to receive (slippage protection)
    pub fn swap_x(
        ctx: Context<SwapCoinX>,
        coin_in_amount: u64,
        coin_y_min_value: u64,
    ) -> Result<()> {
        swap_x::handle(ctx, coin_in_amount, coin_y_min_value)
    }

    /// Preview swap: buying meme tokens with SOL
    /// Returns expected amounts without executing trade
    ///
    /// # Arguments
    /// * `coin_in_amount` - Amount of SOL to spend
    /// * `coin_x_min_value` - Minimum meme tokens to receive (slippage protection)
    pub fn get_swap_y_amt(
        ctx: Context<GetSwapYAmt>,
        coin_in_amount: u64,
        coin_x_min_value: u64,
    ) -> Result<()> {
        get_swap_y_amt::handle(ctx, coin_in_amount, coin_x_min_value)
    }

    /// Execute swap: buy meme tokens with SOL
    /// Direct transfer to user's wallet + points rewards for referrers
    /// 🌟 Automatically triggers migration when 80% threshold reached
    ///
    /// # Arguments
    /// * `coin_in_amount` - Amount of SOL to spend
    /// * `coin_x_min_value` - Minimum meme tokens to receive (slippage protection)
    pub fn swap_y(
        ctx: Context<SwapCoinY>,
        coin_in_amount: u64,
        coin_x_min_value: u64,
    ) -> Result<()> {
        swap_y::handle(ctx, coin_in_amount, coin_x_min_value)
    }

    /// Send airdrop funds to designated recipient
    /// Only callable by authorized airdrop distributors
    pub fn send_airdrop_funds(ctx: Context<SendAirdropFunds>) -> Result<()> {
        send_airdrop_funds::handle(ctx)
    }

    // ===== Migration Functions =====

    /// 🌟 Migrate bonding curve liquidity to Raydium CPMM
    ///
    /// Graduates the bonding curve to a full AMM when threshold is reached:
    /// - Triggers when 80% of trading tokens are sold (552M/690M)
    /// - Creates new Raydium CPMM pool via official CPI
    /// - Migrates 95% of remaining liquidity to AMM
    /// - Keeps 5% for continued bonding curve trading
    /// - Enables deeper liquidity and price stability
    ///
    /// # Migration Process
    /// 1. Validates 80% threshold reached
    /// 2. Locks bonding curve pool
    /// 3. Transfers tokens to creator accounts
    /// 4. Calls Raydium CPMM initialize via CPI
    /// 5. Updates pool state and emits event

    /// # Requirements
    /// - Pool must have reached 80% sell threshold
    /// - Pool must not be already migrated
    /// - Meme token key must be < quote token key (Raydium requirement)
    /// - All Raydium accounts properly derived
    pub fn migrate_to_raydium(ctx: Context<MigrateToRaydium>) -> Result<()> {
        migrate_to_raydium::handle(ctx)
    }
}
