use crate::libraries::MulDiv;
use anchor_lang::prelude::*;

pub const MEME_FEE: u64 = 0; // 0%
pub const FEE: u64 = 10_000_000; // 1%
pub const FEE_PRECISION: u64 = 1_000_000_000;

#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Debug, Eq, PartialEq, Default, InitSpace,
)]
pub struct Fees {
    pub fee_meme_percent: u64,
    pub fee_quote_percent: u64,
}

impl Fees {
    pub fn get_fee_meme_amount(&self, amount: u64) -> Result<u64> {
        get_fee_amount(amount, self.fee_meme_percent)
    }

    pub fn get_fee_quote_amount(&self, amount: u64) -> Result<u64> {
        get_fee_amount(amount, self.fee_quote_percent)
    }
}

pub fn get_fee_amount(x: u64, percent: u64) -> Result<u64> {
    Ok(x.mul_div_ceil(percent, FEE_PRECISION).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*; // This imports everything from the parent module

    #[test]
    fn test_basic_fee_calculation() {
        // ARRANGE: Set up your test data
        let amount = 1000; // We're testing with 1000 tokens
        let expected_fee = 1; // 1% of 1000 = 10

        // Call the function for testing
        let actual_fee = get_fee_amount(amount, FEE).unwrap();

        // Check if the result is what is expected
        // This is a test to ensure the fee calculation is correct
        assert_eq!(actual_fee, expected_fee);

        // Add a helpful message for when tests fail
        assert_eq!(
            actual_fee, expected_fee,
            "Expected 1% fee of {} to be {}, but got {}",
            amount, expected_fee, actual_fee
        );

        // EXTRA CHECKS: Test the math makes sense
        assert!(
            actual_fee < amount,
            "Fee should always be less than original amount"
        );
        println!(
            "✅ Test passed! {} tokens with 1% fee = {} fee",
            amount, actual_fee
        );
    }
}
