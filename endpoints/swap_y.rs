/// Import necessary constants from the crate
use crate::consts::{POINTS_MINT, POINTS_PDA};
/// Import error handling
use crate::err::AmmError;
/// Import math utilities
use crate::libraries::MulDiv;
/// Import bonding curve pool model
use crate::models::bound::BoundPool;
/// Import points epoch model
use crate::models::points_epoch::PointsEpoch;
/// Import Anchor lang prelude
use anchor_lang::prelude::*;
/// Import SPL token program types
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
/// Import min function for points calculation
use std::cmp::min;

/// Account validation struct for swapping SOL for meme tokens
#[derive(Accounts)]
#[instruction(coin_in_amount: u64, coin_x_min_value: u64)]
pub struct SwapCoinY<'info> {
    /// The pool account that will be modified during the swap
    #[account(mut)]
    pool: Account<'info, BoundPool>,
    
    /// The pool's meme token vault that holds meme tokens
    #[account(
        mut,
        constraint = pool.meme_reserve.vault == meme_vault.key()
    )]
    meme_vault: Account<'info, TokenAccount>,
    
    /// The pool's quote token vault that holds SOL
    #[account(
        mut,
        constraint = pool.quote_reserve.vault == quote_vault.key()
    )]
    quote_vault: Account<'info, TokenAccount>,
    
    /// The user's SOL token account that will send tokens
    #[account(mut)]
    user_sol: Account<'info, TokenAccount>,
    
    /// The user's meme token account that will receive tokens directly
    #[account(
        mut,
        constraint = user_meme.mint == pool.meme_reserve.mint @ AmmError::InvalidTokenMints,
        constraint = user_meme.owner == owner.key()
    )]
    user_meme: Account<'info, TokenAccount>,
    
    /// The user's points token account that will receive points
    #[account(
        mut,
        token::mint = points_mint,
        token::authority = owner,
    )]
    user_points: Account<'info, TokenAccount>,
    
    /// Optional referrer points account to receive referral points
    #[account(
        mut,
        token::mint = points_mint,
        constraint = referrer_points.owner != user_points.owner
    )]
    referrer_points: Option<Account<'info, TokenAccount>>,
    
    /// The current points epoch account with points rate info
    points_epoch: Account<'info, PointsEpoch>,
    
    /// The points token mint account
    #[account(mut, constraint = points_mint.key() == POINTS_MINT.key())]
    points_mint: Account<'info, Mint>,
    
    /// The points PDA token account that holds points to distribute
    #[account(
        mut,
        token::mint = points_mint,
        token::authority = points_pda
    )]
    points_acc: Account<'info, TokenAccount>,
    
    /// The owner/signer of the transaction
    #[account(mut)]
    owner: Signer<'info>,
    
    /// PDA signer for points distribution
    /// CHECK: pda signer
    #[account(seeds = [POINTS_PDA], bump)]
    points_pda: AccountInfo<'info>,
    
    /// PDA signer for the pool
    /// CHECK: pda signer
    #[account(seeds = [BoundPool::SIGNER_PDA_PREFIX, pool.key().as_ref()], bump)]
    pool_signer_pda: AccountInfo<'info>,
    
    /// The SPL token program
    token_program: Program<'info, Token>,
}

impl<'info> SwapCoinY<'info> {
    /// Helper function to create CPI context for transferring SOL from user
    fn send_user_tokens(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_sol.to_account_info(),
            to: self.quote_vault.to_account_info(),
            authority: self.owner.to_account_info(),
        };

        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    /// Helper function to create CPI context for transferring meme tokens to user
    fn send_meme_to_user(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.meme_vault.to_account_info(),
            to: self.user_meme.to_account_info(),
            authority: self.pool_signer_pda.to_account_info(),
        };

        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    /// Helper function to create CPI context for transferring points to user
    fn send_user_points(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.points_acc.to_account_info(),
            to: self.user_points.to_account_info(),
            authority: self.points_pda.to_account_info(),
        };

        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

/// Handler function for swapping SOL for meme tokens
///
/// # Arguments
/// * `ctx` - The context containing all required accounts
/// * `coin_in_amount` - Amount of SOL to swap
/// * `coin_x_min_value` - Minimum amount of meme tokens to receive
pub fn handle(
    ctx: Context<SwapCoinY>,
    coin_in_amount: u64,
    coin_x_min_value: u64,
) -> Result<()> {
    /// Get accounts from context
    let accs = ctx.accounts;

    /// Check that input amount is not zero
    if coin_in_amount == 0 {
        return Err(error!(AmmError::NoZeroTokens));
    }

    /// Check that pool is not locked
    if accs.pool.locked {
        return Err(error!(AmmError::PoolIsLocked));
    }

    /// Calculate swap amounts
    let swap_amount = accs
        .pool
        .swap_amounts(coin_in_amount, coin_x_min_value, true);

    /// Transfer SOL from user to pool
    token::transfer(
        accs.send_user_tokens(),
        swap_amount.amount_in + swap_amount.admin_fee_in,
    )?;

    /// Create pool signer PDA seeds for meme token transfer
    let pool_signer_seeds = &[
        BoundPool::SIGNER_PDA_PREFIX,
        &accs.pool.key().to_bytes()[..],
        &[ctx.bumps.pool_signer_pda],
    ];

    /// Transfer meme tokens directly to user's wallet
    token::transfer(
        accs.send_meme_to_user()
            .with_signer(&[&pool_signer_seeds[..]]),
        swap_amount.amount_out,
    )?;

    /// Create points PDA signer seeds
    let point_pda: &[&[u8]] = &[POINTS_PDA, &[ctx.bumps.points_pda]];
    let point_pda_seeds = &[&point_pda[..]];

    /// Get available points amount
    let available_points_amt = accs.points_acc.amount;

    /// Calculate points for swap
    let points = get_swap_points(
        swap_amount.amount_in + swap_amount.admin_fee_in,
        &accs.points_epoch,
    );
    /// Clamp points to available amount
    let clamped_points = min(available_points_amt, points);

    /// Transfer points if available
    if clamped_points > 0 {
        /// Check if referrer account exists
        if let Some(referrer) = &mut accs.referrer_points {
            /// Referrer gets ALL the points (100%, not 25%)
            let referrer_points = clamped_points; // Give full amount to referrer

            /// Clamp to available amount in points pool
            let clamped_referrer_points = min(available_points_amt, referrer_points);

            /// If there are points to give to referrer
            if clamped_referrer_points > 0 {
                /// Setup transfer accounts for referrer
                let cpi_accounts = Transfer {
                    from: accs.points_acc.to_account_info(),
                    to: referrer.to_account_info(),
                    authority: accs.points_pda.to_account_info(),
                };

                /// Get token program account
                let cpi_program = accs.token_program.to_account_info();

                /// Transfer ALL points to referrer only
                token::transfer(
                    CpiContext::new(cpi_program, cpi_accounts).with_signer(point_pda_seeds),
                    clamped_referrer_points,
                )?;

                /// Log referrer reward
                msg!(
                    "Referrer received {} points for successful referral!",
                    clamped_referrer_points
                );
            }
        } else {
            /// No referrer = no points distributed at all!
            /// This incentivizes users to use referral codes
            msg!("No referrer provided - no points distributed. Use a referral code to reward the community!");
        }
    }

    /// Get mutable reference to pool
    let pool = &mut accs.pool;

    /// Update pool admin fees
    pool.admin_fees_quote += swap_amount.admin_fee_in;
    pool.admin_fees_meme += swap_amount.admin_fee_out;

    /// Update pool reserves
    pool.quote_reserve.tokens += swap_amount.amount_in;
    pool.meme_reserve.tokens -= swap_amount.amount_out + swap_amount.admin_fee_out;

    /// Lock pool if meme tokens depleted
    if pool.meme_reserve.tokens == 0 {
        pool.locked = true;
    };

    /// Log swap amounts
    msg!(
        "swapped_in: {}\n swapped_out: {}",
        swap_amount.amount_in,
        swap_amount.amount_out
    );

    Ok(())
}

/// Calculate points earned for a swap
///
/// # Arguments
/// * `buy_amount` - Amount of SOL being swapped
/// * `points_epoch` - Current points epoch with points rate
pub fn get_swap_points(buy_amount: u64, points_epoch: &PointsEpoch) -> u64 {
    buy_amount
        .mul_div_floor(
            points_epoch.points_per_sol_num,
            points_epoch.points_per_sol_denom,
        )
        .unwrap()
}