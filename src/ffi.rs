//! WebAssembly exports. Plain numbers only, so the module loads with nothing
//! but the standard `WebAssembly` API: no generated glue code.
//!
//! Amounts cross the boundary as `f64` holding whole cents, which JavaScript
//! represents exactly up to 2^53; anything fractional or out of range is
//! rejected rather than rounded.

use crate::{settle, Cents, Limits, SettleError, Settlement, MAX_CENTS};

/// Returned by [`pc_settle`] when no price fits both limits.
pub const NO_DEAL: f64 = -1.0;
/// Returned when an amount is not a whole number of cents in range.
pub const INVALID_AMOUNT: f64 = -2.0;
/// Returned when the seller's minimum is above the asking price.
pub const MIN_ABOVE_ASK: f64 = -3.0;

/// Version of this numeric interface; bumped on any breaking change.
#[unsafe(no_mangle)]
pub extern "C" fn pc_abi_version() -> u32 {
    2
}

fn cents(x: f64, allow_zero: bool) -> Option<Cents> {
    let low_ok = if allow_zero { x >= 0.0 } else { x > 0.0 };
    (x.is_finite() && x.fract() == 0.0 && low_ok && x <= MAX_CENTS as f64).then_some(x as Cents)
}

/// Settle one pairing. `floor` is the best price this buyer already walked
/// away from for this listing, or 0. Returns the price in cents, or a negative
/// code: [`NO_DEAL`], [`INVALID_AMOUNT`] or [`MIN_ABOVE_ASK`].
#[unsafe(no_mangle)]
pub extern "C" fn pc_settle(ask: f64, seller_min: f64, buyer_max: f64, floor: f64) -> f64 {
    let (Some(ask), Some(seller_min), Some(buyer_max), Some(floor)) = (
        cents(ask, false),
        cents(seller_min, false),
        cents(buyer_max, false),
        cents(floor, true),
    ) else {
        return INVALID_AMOUNT;
    };
    match settle(Limits {
        ask,
        seller_min,
        buyer_max,
        floor,
    }) {
        Ok(Settlement::Deal { price, .. }) => price as f64,
        Ok(Settlement::NoDeal) => NO_DEAL,
        Err(SettleError::MinAboveAsk) => MIN_ABOVE_ASK,
        Err(_) => INVALID_AMOUNT,
    }
}
