//! WebAssembly exports. Plain numbers only, so the module loads with nothing
//! but the standard `WebAssembly` API: no generated glue code.
//!
//! Amounts cross the boundary as `f64` holding whole cents, which JavaScript
//! represents exactly up to 2^53; anything fractional or out of range is
//! rejected rather than rounded.

use crate::{settle_with, Cents, Limits, SettleError, Settlement, Ticks, MAX_CENTS};

/// Returned by [`pc_settle`] when no price fits both limits.
pub const NO_DEAL: f64 = -1.0;
/// Returned when an amount is not a positive whole number of cents in range,
/// or a custom tick schedule is invalid.
pub const INVALID_AMOUNT: f64 = -2.0;
/// Returned when the seller's minimum is above the asking price.
pub const MIN_ABOVE_ASK: f64 = -3.0;

/// Version of this numeric interface; bumped on any breaking change.
#[unsafe(no_mangle)]
pub extern "C" fn pc_abi_version() -> u32 {
    1
}

fn cents(x: f64) -> Option<Cents> {
    (x.is_finite() && x.fract() == 0.0 && x > 0.0 && x <= MAX_CENTS as f64).then_some(x as Cents)
}

fn run(ask: f64, seller_min: f64, buyer_max: f64, ticks: Ticks) -> f64 {
    let (Some(ask), Some(seller_min), Some(buyer_max)) =
        (cents(ask), cents(seller_min), cents(buyer_max))
    else {
        return INVALID_AMOUNT;
    };
    match settle_with(
        Limits {
            ask,
            seller_min,
            buyer_max,
        },
        ticks,
    ) {
        Ok(Settlement::Deal { price, .. }) => price as f64,
        Ok(Settlement::NoDeal) => NO_DEAL,
        Err(SettleError::MinAboveAsk) => MIN_ABOVE_ASK,
        Err(_) => INVALID_AMOUNT,
    }
}

/// Settle with the USD tick schedule. Returns the price in cents, or a
/// negative code: [`NO_DEAL`], [`INVALID_AMOUNT`] or [`MIN_ABOVE_ASK`].
#[unsafe(no_mangle)]
pub extern "C" fn pc_settle(ask: f64, seller_min: f64, buyer_max: f64) -> f64 {
    run(ask, seller_min, buyer_max, Ticks::USD)
}

/// Settle with a custom tick schedule (all values in cents).
#[unsafe(no_mangle)]
pub extern "C" fn pc_settle_with(
    ask: f64,
    seller_min: f64,
    buyer_max: f64,
    small_below: f64,
    small_tick: f64,
    large_tick: f64,
) -> f64 {
    let (Some(small_tick), Some(large_tick)) = (cents(small_tick), cents(large_tick)) else {
        return INVALID_AMOUNT;
    };
    let small_below = if small_below == 0.0 {
        0
    } else if let Some(c) = cents(small_below) {
        c
    } else {
        return INVALID_AMOUNT;
    };
    run(
        ask,
        seller_min,
        buyer_max,
        Ticks {
            small_below,
            small_tick,
            large_tick,
        },
    )
}
