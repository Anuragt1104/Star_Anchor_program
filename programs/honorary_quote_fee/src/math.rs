use anchor_lang::prelude::*;

use crate::errors::HonoraryQuoteFeeError;

pub fn mul_div_floor_u128(a: u128, b: u128, denominator: u128) -> Result<u128> {
    require!(denominator != 0, HonoraryQuoteFeeError::ArithmeticOverflow);
    let product = a
        .checked_mul(b)
        .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;
    Ok(product
        .checked_div(denominator)
        .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?)
}

pub fn u128_to_u64(value: u128) -> Result<u64> {
    if value > u64::MAX as u128 {
        return err!(HonoraryQuoteFeeError::ArithmeticOverflow);
    }
    Ok(value as u64)
}

pub fn saturating_sub_u64(lhs: u64, rhs: u64) -> u64 {
    lhs.saturating_sub(rhs)
}
