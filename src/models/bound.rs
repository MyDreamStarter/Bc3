/// Import necessary modules from crate
use crate::{
    consts::DECIMALS_S,
    err::AmmError,
    math::utils::{multiply_divide, CheckedMath, CheckedMath256},
};

/// Import Anchor lang prelude for Solana program development
use anchor_lang::prelude::*;

/// Import Solana program pubkey type
use solana_program::pubkey::Pubkey;

/// Import SPL math library for U256 type
use spl_math::uint::U256;

/// Import standard library components
use std::cmp::min;

/// Import related models
use super::{fees::Fees, Reserve, SwapAmount};

/// Account struct representing a bonding curve pool
#[account]
#[derive(InitSpace)]
pub struct BoundPool {
    /// Reserve account for meme tokens
    pub meme_reserve: Reserve,
    /// Reserve account for quote tokens (SOL)
    pub quote_reserve: Reserve,
    /// Admin fee balance for meme tokens
    pub admin_fees_meme: u64,
    /// Admin fee balance for quote tokens
    pub admin_fees_quote: u64,
    /// Public key of fee vault for quote tokens
    pub fee_vault_quote: Pubkey,
    /// Public key of pool creator
    pub creator_addr: Pubkey,
    /// Fee configuration
    pub fees: Fees,
    /// Pool configuration parameters
    pub config: Config,
    /// Number of airdropped tokens
    pub airdropped_tokens: u64,
    /// Flag indicating if pool is locked
    pub locked: bool,
    /// Vesting period duration
    pub vesting_period: i64,
}

impl BoundPool {
    /// Prefix for pool PDA derivation
    pub const POOL_PREFIX: &'static [u8; 10] = b"bound_pool";
    /// Prefix for signer PDA derivation
    pub const SIGNER_PDA_PREFIX: &'static [u8; 6] = b"signer";
}

/// Struct holding decimal configuration values
#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Debug, Eq, PartialEq, Default, InitSpace,
)]
pub struct Decimals {
    /// Alpha decimal precision
    pub alpha: u128,
    /// Beta decimal precision (for negative intercept)
    pub beta: u128,
    /// Quote token decimal precision
    pub quote: u64,
}

/// Struct holding pool configuration parameters
#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Debug, Eq, PartialEq, Default, InitSpace,
)]
pub struct Config {
    /// Positive slope value (alpha)
    pub alpha_abs: u128,
    /// Negative beta value (y-intercept, stored as absolute value)
    pub beta: u128,
    /// Price factor numerator
    pub price_factor_num: u64,
    /// Price factor denominator
    pub price_factor_denom: u64,
    /// Maximum SOL amount in raw denomination
    pub gamma_s: u64,
    /// Maximum meme token amount in raw denomination
    pub gamma_m: u64,
    /// Maximum LP token amount in raw denomination
    pub omega_m: u64,
    /// Decimal configuration values
    pub decimals: Decimals,
}

impl BoundPool {
    pub fn swap_amounts(
        &self,
        coin_in_amount: u64,
        coin_out_min_value: u64,
        buy_meme: bool,
    ) -> SwapAmount {
        if buy_meme {
            self.buy_meme_swap_amounts(coin_in_amount, coin_out_min_value)
                .unwrap()
        } else {
            self.sell_meme_swap_amounts(coin_in_amount, coin_out_min_value)
                .unwrap()
        }
    }

    fn buy_meme_swap_amounts(&self, delta_s: u64, min_delta_m: u64) -> Result<SwapAmount> {
        let (m_t0, s_t0) = self.balances();

        let p = &self.config;

        let max_delta_s = p.gamma_s - s_t0;

        let admin_fee_in = self.fees.get_fee_quote_amount(delta_s).unwrap();
        let is_max = delta_s - admin_fee_in >= max_delta_s;

        let net_delta_s = min(delta_s - admin_fee_in, max_delta_s);

        let delta_m = if is_max {
            m_t0
        } else {
            self.compute_delta_m(s_t0, s_t0 + net_delta_s)?
        };

        let admin_fee_out = self.fees.get_fee_meme_amount(delta_m).unwrap();
        let net_delta_m = delta_m - admin_fee_out;

        if net_delta_m < min_delta_m {
            return Err(error!(AmmError::SlippageExceeded));
        }

        Ok(SwapAmount {
            amount_in: net_delta_s,
            amount_out: net_delta_m,
            admin_fee_in,
            admin_fee_out,
        })
    }

    fn sell_meme_swap_amounts(&self, delta_m: u64, min_delta_s: u64) -> Result<SwapAmount> {
        let (m_b, s_b) = self.balances();

        let p = &self.config;

        let max_delta_m = p.gamma_m - m_b;

        let admin_fee_in = self.fees.get_fee_meme_amount(delta_m).unwrap() * 2;
        let is_max = delta_m - admin_fee_in >= max_delta_m;

        let net_delta_m = min(delta_m - admin_fee_in, max_delta_m);

        let delta_s = if is_max {
            s_b
        } else {
            self.compute_delta_s(s_b, net_delta_m)?
        };

        let admin_fee_out = self.fees.get_fee_quote_amount(delta_s).unwrap() * 2;
        let net_delta_s = delta_s - admin_fee_out;

        if net_delta_s < min_delta_s {
            return Err(error!(AmmError::SlippageExceeded));
        }

        Ok(SwapAmount {
            amount_in: net_delta_m,
            amount_out: net_delta_s,
            admin_fee_in,
            admin_fee_out,
        })
    }

    /// CHANGED: Updated for positive slope bonding curve
    /// Formula: price = +alpha_abs * supply - beta (negative intercept)
    pub fn compute_delta_m(&self, s_a: u64, s_b: u64) -> Result<u64> {
        let s_a = s_a as u128;
        let s_b = s_b as u128;

        let alpha_abs = self.config.alpha_abs;
        let beta = self.config.beta;
        let alpha_decimals = self.config.decimals.alpha;
        let beta_decimals = self.config.decimals.beta;

        return match delta_m1_positive_strategy(
            alpha_abs,
            beta,
            alpha_decimals,
            beta_decimals,
            s_a,
            s_b,
        ) {
            Some(delta_m) => Ok(delta_m as u64),
            None => {
                match delta_m2_positive_strategy(
                    alpha_abs,
                    beta,
                    alpha_decimals,
                    beta_decimals,
                    s_a,
                    s_b,
                ) {
                    Some(delta_m) => Ok(delta_m as u64),
                    None => Err(error!(AmmError::MathOverflow)),
                }
            }
        };
    }

    /// CHANGED: Updated for positive slope bonding curve
    pub fn compute_delta_s(&self, s_b: u64, delta_m: u64) -> Result<u64> {
        let s_b = s_b as u128;
        let delta_m = delta_m as u128;

        let alpha_abs = self.config.alpha_abs;
        let beta = self.config.beta;
        let alpha_decimals = self.config.decimals.alpha;
        let beta_decimals = self.config.decimals.beta;

        match delta_s_positive_strategy(
            alpha_abs,
            beta,
            alpha_decimals,
            beta_decimals,
            s_b,
            delta_m,
        ) {
            Some(delta_s) => Ok(delta_s as u64),
            None => Err(error!(AmmError::MathOverflow)),
        }
    }

    fn balances(&self) -> (u64, u64) {
        (self.meme_reserve.tokens, self.quote_reserve.tokens)
    }
}

/// CHANGED: Updated for positive slope calculation
pub fn compute_alpha_abs(
    gamma_s: u128,
    gamma_s_denom: u128,
    gamma_m: u128,
    omega_m: u128,
    price_factor_num: u64,
    price_factor_denom: u64,
) -> Result<(u128, u128)> {
    check_slope(gamma_m, omega_m, price_factor_num, price_factor_denom)?;

    let left = omega_m
        .checked_mul(price_factor_num as u128)
        .checked_div(price_factor_denom as u128)
        .unwrap();

    // For positive slope: price increases with supply
    // Formula: alpha = 2 * (left - gamma_m) * gamma_s_denom^2 / gamma_s^2
    let num = U256::from(2 * (left - gamma_m)) * U256::from(gamma_s_denom * gamma_s_denom);
    let denom = U256::from(gamma_s * gamma_s);

    if num <= denom {
        return Err(error!(AmmError::EGammaSAboveRelativeLimit));
    }

    // Calculate the scale (order of magnitude) of numerator and denominator
    let num_scale = compute_scale(num.as_u128());
    let denom_scale = compute_scale(denom.as_u128());

    // Get the difference in scales to determine required decimal precision
    let net_scale = num_scale - denom_scale;

    // Convert net_scale to appropriate decimal precision for alpha
    let alpha_decimals = U256::from(compute_decimals(net_scale)?);

    // For positive slope, we keep the sign positive
    Ok((
        ((num * alpha_decimals) / denom).as_u128(),
        alpha_decimals.as_u128(),
    ))
}

pub fn compute_decimals(scale: u64) -> Result<u128> {
    match scale {
        0..=4 => return Err(error!(AmmError::EScaleTooLow)),
        5 => Ok(100_000_000),
        6 => Ok(10_000_000),
        7 => Ok(1_000_000),
        8 => Ok(100_000),
        9 => Ok(10_000),
        10 => Ok(1_000),
        11 => Ok(100),
        12 => Ok(10),
        _ => Ok(1), // If scale is above 12
    }
}

/// CHANGED: Updated for positive slope with negative intercept
pub fn compute_beta(
    gamma_s: u128,
    gamma_s_denom: u128,
    gamma_m: u128,
    omega_m: u128,
    price_factor_num: u64,
    price_factor_denom: u64,
    beta_decimals: u128,
) -> Result<u128> {
    check_intercept(gamma_m, omega_m, price_factor_num, price_factor_denom)?;

    let left = omega_m
        .checked_mul(price_factor_num as u128)
        .checked_div(price_factor_denom as u128)
        .unwrap();

    // For positive slope with negative intercept:
    // beta = (left - 2 * gamma_m) * gamma_s_denom / gamma_s
    // This will be negative, but we store absolute value
    let right = 2 * gamma_m;

    let num = (left - right) * gamma_s_denom;
    let denom = gamma_s;

    Ok((num * beta_decimals) / denom)
}

/// CHANGED: For positive slope bonding curve - price increases as supply increases
pub fn check_slope(
    gamma_m: u128,
    omega_m: u128,
    price_factor_num: u64,
    price_factor_denom: u64,
) -> Result<()> {
    let pfo = omega_m
        .checked_mul(price_factor_num as u128)
        .checked_div(price_factor_denom as u128)
        .unwrap();

    // For positive slope: omega_m * price_factor must be GREATER than gamma_m
    // This ensures positive slope (price increases with supply)
    if pfo <= gamma_m {
        return Err(error!(AmmError::BondingCurveMustBePositivelySloped));
    }

    Ok(())
}

/// CHANGED: For positive slope bonding curve - requires negative intercept
pub fn check_intercept(
    gamma_m: u128,
    omega_m: u128,
    price_factor_num: u64,
    price_factor_denom: u64,
) -> Result<()> {
    let omp = omega_m
        .checked_mul(price_factor_num as u128)
        .checked_div(price_factor_denom as u128)
        .unwrap();

    // For positive slope: intercept must be negative
    // This means omega_m * price_factor < 2 * gamma_m
    if 2 * gamma_m >= omp {
        return Err(error!(AmmError::BondingCurveInterceptMustBeNegative));
    }

    Ok(())
}

fn compute_scale(num_: u128) -> u64 {
    let mut num = num_;

    return if num == 0 {
        1
    } else {
        let mut scale = 1;

        while num >= 10 {
            num = num / 10;
            scale += 1;
        }

        scale
    };
}

/// CHANGED: New strategy for positive slope delta_s calculation
fn delta_s_positive_strategy(
    alpha_abs: u128,
    beta: u128,
    alpha_decimals: u128,
    beta_decimals: u128,
    s_b: u128,
    delta_m: u128,
) -> Option<u128> {
    let alpha_abs = U256::from(alpha_abs);
    let beta = U256::from(beta);
    let alpha_decimals = U256::from(alpha_decimals);
    let beta_decimals = U256::from(beta_decimals);
    let s_b = U256::from(s_b);
    let delta_m = U256::from(delta_m);
    let decimals_s = U256::from(DECIMALS_S);

    // For positive slope: price = +alpha_abs * supply - beta
    // Solving for delta_s when we know delta_m
    // delta_m = alpha_abs * delta_s + (other terms)

    let u = U256::from(2)
        .checked_mul(beta)
        .checked_mul(alpha_decimals)
        .checked_mul(decimals_s)
        .checked_add_(
            U256::from(2)
                .checked_mul(alpha_abs)
                .checked_mul(s_b)
                .checked_mul(beta_decimals),
        )?;

    let v = alpha_decimals
        .checked_mul(beta_decimals)
        .checked_mul(decimals_s)?;

    let w = U256::from(8).checked_mul(delta_m).checked_mul(alpha_abs)?;

    let a = compute_a_positive(u, alpha_decimals, w, v, U256::from(1))?;

    let b = v
        .checked_pow(U256::from(2))
        .checked_mul(alpha_decimals)
        .sqrt()?;

    let num_1 = vec![decimals_s, alpha_decimals, a, v];
    let num_2 = vec![decimals_s, alpha_decimals, u, b];
    let denom_ = vec![U256::from(2), alpha_abs, b, v];

    let left = multiply_divide(num_1, denom_.clone());
    let right = multiply_divide(num_2, denom_);

    // For positive slope, we add instead of subtract
    left.checked_add_(right).map(|value| value.as_u128())
}

fn compute_a_positive(
    u: U256,
    alpha_decimals: U256,
    w: U256,
    v: U256,
    scale: U256,
) -> Option<U256> {
    let left = u
        .checked_div(scale)
        .checked_mul(u)
        .checked_mul(alpha_decimals);

    let right = v.checked_div(scale).checked_mul(v).checked_mul(w);

    let result = left
        .checked_add_(right)
        .sqrt()
        .checked_mul(scale.integer_sqrt());

    match result {
        Some(value) => Some(value),
        None => compute_a_positive(
            u,
            alpha_decimals,
            w,
            v,
            scale.checked_mul(U256::from(100)).unwrap(),
        ),
    }
}

/// CHANGED: New strategy for positive slope delta_m calculation (method 2)
// fn delta_m2_positive_strategy(
//     alpha_abs: u128,
//     beta: u128,
//     alpha_decimals: u128,
//     beta_decimals: u128,
//     s_a: u128,
//     s_b: u128,
// ) -> Option<u128> {
//     // For positive slope: price = +alpha_abs * supply - beta
//     let left = (beta * 2)
//         .checked_mul(DECIMALS_S)
//         .checked_mul(alpha_decimals)
//         .checked_mul(s_b - s_a)?;

//     let right = alpha_abs
//         .checked_mul(beta_decimals)
//         .checked_mul_(s_b.checked_pow(2).checked_sub_(s_a.checked_pow(2))?)?;

//     let denom = (2 * alpha_decimals)
//         .checked_mul(beta_decimals)
//         .checked_mul_(DECIMALS_S.checked_pow(2))?;

//     // For positive slope, we ADD the terms instead of subtract
//     left.checked_add_(right)?.checked_div_(denom)
// }

/// CHANGED: New strategy for positive slope delta_m calculation (method 2)
fn delta_m2_positive_strategy(
    alpha_abs: u128,
    beta: u128,
    alpha_decimals: u128,
    beta_decimals: u128,
    s_a: u128,
    s_b: u128,
) -> Option<u128> {
    // For positive slope: price = +alpha_abs * supply - beta
    let left = (beta * 2)
        .checked_mul(DECIMALS_S)
        .checked_mul(alpha_decimals)
        .checked_mul(s_b - s_a)?;

    // FIX: Unwrap both s_a and s_b powers, then subtract
    let s_b_squared = s_b.checked_pow(2)?;
    let s_a_squared = s_a.checked_pow(2)?;
    let power_diff = s_b_squared.checked_sub(s_a_squared)?;

    // FIX: Use checked_mul instead of checked_mul_
    let right = alpha_abs
        .checked_mul(beta_decimals)
        .checked_mul(power_diff)?;

    // FIX: Use checked_mul instead of checked_mul_
    let denom = (2 * alpha_decimals)
        .checked_mul(beta_decimals)
        .checked_mul(DECIMALS_S.checked_pow(2)?)?;

    // For positive slope, we ADD the terms instead of subtract
    left.checked_add(right)?.checked_div(denom)
}

/// CHANGED: New strategy for positive slope delta_m calculation (method 1)
fn delta_m1_positive_strategy(
    alpha_abs: u128,
    beta: u128,
    alpha_decimals: u128,
    beta_decimals: u128,
    s_a: u128,
    s_b: u128,
) -> Option<u128> {
    let left_num = s_b.checked_sub(s_a)?.checked_mul(beta)?;

    let left_denom = beta_decimals.checked_mul(DECIMALS_S)?;

    let left = Some(left_num).checked_div_(Some(left_denom))?;

    let s_b_squared = s_b.checked_pow(2)?;
    let s_a_squared = s_a.checked_pow(2)?;
    let power_diff = s_b_squared.checked_sub(s_a_squared)?;
    let decimals_s_squared = DECIMALS_S.checked_pow(2)?;

    let right = power_diff
        .checked_mul(alpha_abs)
        .checked_div(decimals_s_squared)?
        .checked_div(2 * alpha_decimals)?;

    // For positive slope, we ADD the terms instead of subtract
    Some(left).checked_add_(Some(right))
}
