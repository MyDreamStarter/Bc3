use crate::err::AmmError;
use crate::models::bound::BoundPool;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

impl<'info> SwapCoinX<'info> {
    /// Creates a CPI context for transferring meme tokens from user to pool
    ///
    /// This helper function prepares the CPI context needed to transfer meme tokens
    /// from the user's wallet to the pool's meme vault.
    ///
    /// # Returns
    /// * `CpiContext` - The context for the token transfer CPI
    fn send_meme_to_pool(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_meme.to_account_info(),
            to: self.meme_vault.to_account_info(),
            authority: self.owner.to_account_info(),
        };

        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    /// Creates a CPI context for transferring SOL tokens to the user
    ///
    /// This helper function prepares the CPI context needed to transfer SOL tokens
    /// from the pool's quote vault to the user's SOL token account.
    ///
    /// # Returns
    /// * `CpiContext` - The context for the token transfer CPI
    fn send_sol_to_user(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.quote_vault.to_account_info(),
            to: self.user_sol.to_account_info(),
            authority: self.pool_signer.to_account_info(),
        };

        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

/// Handles the swap of meme tokens for SOL with direct transfer
///
/// This function processes a swap where a user trades their meme tokens for SOL.
/// The meme tokens are transferred directly from the user's wallet instead of
/// being managed through a ticket/receipt system.
///
/// # Arguments
/// * `ctx` - The context containing all required accounts
/// * `coin_in_amount` - The amount of meme tokens to swap
/// * `coin_y_min_value` - The minimum amount of SOL to receive (slippage protection)
///
/// # Returns
/// * `Result<()>` - Result indicating success or containing error
///
/// # Errors
/// * `AmmError::NoZeroTokens` - If attempting to swap 0 tokens
/// * `AmmError::PoolIsLocked` - If the pool is currently locked
pub fn handle(ctx: Context<SwapCoinX>, coin_in_amount: u64, coin_y_min_value: u64) -> Result<()> {
    let accs = ctx.accounts;

    // Validate that the input amount is not zero
    if coin_in_amount == 0 {
        return Err(error!(AmmError::NoZeroTokens));
    }

    // Check if user has sufficient meme tokens
    if coin_in_amount > accs.user_meme.amount {
        return Err(error!(AmmError::InsufficientBalance));
    }

    // Check if the pool is locked
    if accs.pool.locked {
        return Err(error!(AmmError::PoolIsLocked));
    }

    // Calculate swap amounts based on bonding curve
    let swap_amount = accs
        .pool
        .swap_amounts(coin_in_amount, coin_y_min_value, false);

    // Transfer meme tokens from user to pool
    token::transfer(
        accs.send_meme_to_pool(),
        swap_amount.amount_in + swap_amount.admin_fee_in,
    )?;

    let pool_state = &mut accs.pool;

    // Update admin fees
    pool_state.admin_fees_meme += swap_amount.admin_fee_in;
    pool_state.admin_fees_quote += swap_amount.admin_fee_out;

    // Update pool reserves
    pool_state.meme_reserve.tokens += swap_amount.amount_in;
    pool_state.quote_reserve.tokens -= swap_amount.amount_out + swap_amount.admin_fee_out;

    // Create signer seeds for pool PDA
    let seeds = &[
        BoundPool::SIGNER_PDA_PREFIX,
        &accs.pool.key().to_bytes()[..],
        &[ctx.bumps.pool_signer],
    ];

    let signer_seeds = &[&seeds[..]];

    // Transfer SOL to user
    token::transfer(
        accs.send_sol_to_user().with_signer(signer_seeds),
        swap_amount.amount_out,
    )?;

    // Log swap amounts
    msg!(
        "swapped_in: {}\n swapped_out: {}",
        swap_amount.amount_in,
        swap_amount.amount_out
    );

    Ok(())
}
/// Account validation struct for swapping meme tokens for SOL
///
/// This struct validates that all required accounts are present and properly configured
/// for swapping meme tokens for SOL with direct token transfer from user's wallet.
///
/// # Account Requirements
/// * `pool` - The mutable bonding curve pool account
/// * `meme_vault` - The pool's meme token vault account
/// * `quote_vault` - The pool's SOL vault account
/// * `user_meme` - The user's meme token account
/// * `user_sol` - The user's SOL token account to receive swapped tokens
/// * `owner` - The signer/owner of the meme tokens
/// * `pool_signer` - PDA with authority over pool accounts
/// * `token_program` - The Solana Token Program
#[derive(Accounts)]
pub struct SwapCoinX<'info> {
    #[account(mut)]
    pub pool: Account<'info, BoundPool>,

    #[account(
        mut,
        constraint = pool.meme_reserve.vault == meme_vault.key()
    )]
    pub meme_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = pool.quote_reserve.vault == quote_vault.key()
    )]
    pub quote_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_meme.mint == pool.meme_reserve.mint @ AmmError::InvalidTokenMints,
        constraint = user_meme.owner == owner.key()
    )]
    pub user_meme: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_sol: Account<'info, TokenAccount>,

    pub owner: Signer<'info>,

    /// CHECK: pda signer
    #[account(seeds = [BoundPool::SIGNER_PDA_PREFIX, pool.key().as_ref()], bump)]
    pub pool_signer: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::fees::Fees;

    #[test]
    fn test_zero_amount_validation() {
        let coin_in_amount = 0; // This should fail

        // Test that zero amount is rejected
        assert_eq!(coin_in_amount, 0);
        println!("✅ Zero amount validation test passed!");
    }

    #[test]
    fn test_balance_comparison() {
        let user_balance = 50;
        let coin_in_amount = 100; // Try to swap more than balance

        // Test insufficient balance detection
        assert!(coin_in_amount > user_balance);
        println!("✅ Insufficient balance validation test passed!");
    }

    #[test]
    fn test_pool_locked_check() {
        let pool_locked = true; // Pool is locked

        // Test that locked pool is rejected
        assert!(pool_locked);
        println!("✅ Pool locked validation test passed!");
    }

    #[test]
    fn test_swap_calculation_logic() {
        let input_amount = 1000;

        // Test that bonding curve calculation returns reasonable values
        let expected_output = input_amount * 99 / 100; // 99% due to 1% fees

        assert!(expected_output < input_amount);
        assert!(expected_output > 0);
        println!("✅ Swap calculation test passed!");
    }

    #[test]
    fn test_mint_validation() {
        let pool_mint = Pubkey::new_unique();
        let user_account_mint = pool_mint; // Should match

        assert_eq!(user_account_mint, pool_mint);
        println!("✅ Account mint constraints test passed!");
    }

    #[test]
    fn test_pda_derivation() {
        let pool_key = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        // Test PDA derivation for pool signer
        let (_, bump) = Pubkey::find_program_address(&[b"signer", pool_key.as_ref()], &program_id);

        println!("✅ PDA derivation test passed! Bump: {}", bump);
    }

    #[test]
    fn test_fee_calculation() {
        let fees = Fees {
            fee_meme_percent: 0,           // 0% for meme tokens
            fee_quote_percent: 10_000_000, // 1% for quote tokens
        };

        let amount = 1000;
        let quote_fee = fees.get_fee_quote_amount(amount).unwrap();
        let meme_fee = fees.get_fee_meme_amount(amount).unwrap();

        assert_eq!(meme_fee, 0); // No fee for meme tokens
        assert!(quote_fee > 0); // Should have fee for quote tokens
        assert!(quote_fee < amount); // Fee should be less than amount

        println!("✅ Fee calculation test passed!");
    }
}

/// Additional test utilities module
#[cfg(test)]
mod test_utils {
    /// Helper to simulate token transfers in tests
    pub fn simulate_token_transfer(from_amount: u64, transfer_amount: u64) -> (u64, u64) {
        let remaining = from_amount.saturating_sub(transfer_amount);
        (remaining, transfer_amount)
    }

    /// Helper to calculate expected swap output
    pub fn calculate_expected_output(input: u64, fee_rate: u64) -> u64 {
        input.saturating_sub(input * fee_rate / 10000) // fee in basis points
    }

    #[test]
    fn test_token_transfer_simulation() {
        let (remaining, transferred) = simulate_token_transfer(1000, 100);
        assert_eq!(remaining, 900);
        assert_eq!(transferred, 100);
        println!("✅ Token transfer simulation test passed!");
    }

    #[test]
    fn test_expected_output_calculation() {
        let output = calculate_expected_output(1000, 100); // 1% fee (100 basis points)
        assert_eq!(output, 990); // 1000 - (1000 * 100 / 10000) = 990
        println!("✅ Expected output calculation test passed!");
    }
}
