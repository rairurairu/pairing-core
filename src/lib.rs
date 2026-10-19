//! # pairing-core
//!
//! Price settlement for two-sided marketplaces where software agents negotiate
//! on behalf of people. Each side delegates a private limit once: the seller a
//! minimum, the buyer a maximum. The agents then settle on a price both people
//! have already authorised, without either limit ever reaching the other side.
//!
//! ```
//! use pairing_core::{settle, Limits, Settlement};
//!
//! // Ask $20, seller accepts down to $15, buyer pays up to $18 → $17.
//! let s = settle(Limits { ask: 2000, seller_min: 1500, buyer_max: 1800 }).unwrap();
//! assert_eq!(s, Settlement::Deal { price: 1700, saved_vs_ask: 300 });
//!
//! // No overlap: the refusal carries no data at all.
//! let s = settle(Limits { ask: 3000, seller_min: 2500, buyer_max: 2000 }).unwrap();
//! assert_eq!(s, Settlement::NoDeal);
//! ```
//!
//! ## Guarantees
//!
//! - **Inside both limits:** `seller_min <= price <= min(ask, buyer_max)`.
//! - **Deal exactly when one is possible:** a deal exists iff
//!   `seller_min <= min(ask, buyer_max)`.
//! - **Refusals reveal nothing:** [`Settlement::NoDeal`] has no fields, so a
//!   caller cannot leak how far apart the limits were.
//! - **Monotone:** raising the ask, the seller's minimum or the buyer's maximum
//!   never lowers the price.
//! - **Fair and predictable:** the price is the midpoint of the zone of possible
//!   agreement, rounded half-up to the price tick for that midpoint, then kept
//!   inside the zone.
//! - **Exact:** all amounts are integer minor units (cents); no floating point.
//!
//! Every guarantee is checked exhaustively over a grid of inputs in
//! `tests/invariants.rs`, and `tests/vectors.csv` pins the exact outputs so
//! other implementations can prove they match.

#![deny(unsafe_code)]
#![warn(missing_docs)]

// Exporting unmangled symbols counts as unsafe; it is the only exception.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
mod ffi;

/// An amount in minor currency units (cents for USD).
pub type Cents = i64;

/// The largest amount accepted: $10 trillion in cents. Keeps every
/// intermediate sum far from overflow.
pub const MAX_CENTS: Cents = 1_000_000_000_000_000;

/// Both sides' delegated limits for one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The seller's public asking price.
    pub ask: Cents,
    /// The seller's private minimum. Never shown to the buyer.
    pub seller_min: Cents,
    /// The buyer's private maximum. Never shown to the seller.
    pub buyer_max: Cents,
}

/// The outcome of a settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settlement {
    /// A price both people have authorised.
    Deal {
        /// The agreed price.
        price: Cents,
        /// How far below the public asking price the agreed price is.
        saved_vs_ask: Cents,
    },
    /// No price fits both limits. Deliberately carries no data.
    NoDeal,
}

/// Why a settlement was rejected before it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettleError {
    /// An amount was zero or negative.
    NotPositive,
    /// An amount was above [`MAX_CENTS`].
    TooLarge,
    /// The seller's minimum was above their own asking price.
    MinAboveAsk,
    /// The tick schedule was invalid (see [`Ticks::validate`]).
    BadTicks,
}

impl core::fmt::Display for SettleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NotPositive => "amounts must be positive",
            Self::TooLarge => "amount is too large",
            Self::MinAboveAsk => "the seller's minimum is above the asking price",
            Self::BadTicks => "ticks must be positive and the threshold a multiple of both ticks",
        })
    }
}

impl std::error::Error for SettleError {}

/// Price ticks: a zone whose midpoint is below `small_below` settles on
/// multiples of `small_tick`, any other on multiples of `large_tick`.
///
/// The threshold must lie on both grids. Then every price from the small grid is
/// at most the threshold and every price from the large grid at least it, which
/// is what keeps settlement monotone across the switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticks {
    /// Midpoints at or above this use `large_tick`.
    pub small_below: Cents,
    /// Tick for small amounts.
    pub small_tick: Cents,
    /// Tick for everything else.
    pub large_tick: Cents,
}

impl Ticks {
    /// $0.50 steps below $10, whole dollars from $10 up.
    pub const USD: Ticks = Ticks {
        small_below: 1000,
        small_tick: 50,
        large_tick: 100,
    };

    /// Every cent is a valid price.
    pub const CENT: Ticks = Ticks {
        small_below: 0,
        small_tick: 1,
        large_tick: 1,
    };

    /// The tick for a zone of agreement whose midpoint is `midpoint`.
    pub const fn tick_for(&self, midpoint: Cents) -> Cents {
        if midpoint < self.small_below {
            self.small_tick
        } else {
            self.large_tick
        }
    }

    /// Ticks must be positive, and the threshold a non-negative multiple of both.
    pub const fn validate(&self) -> Result<(), SettleError> {
        let ok = self.small_tick > 0
            && self.large_tick > 0
            && self.small_below >= 0
            && self.small_below % self.small_tick == 0
            && self.small_below % self.large_tick == 0;
        if ok {
            Ok(())
        } else {
            Err(SettleError::BadTicks)
        }
    }
}

impl Default for Ticks {
    fn default() -> Self {
        Self::USD
    }
}

/// Settle with the default [`Ticks::USD`] schedule.
pub fn settle(limits: Limits) -> Result<Settlement, SettleError> {
    settle_with(limits, Ticks::USD)
}

/// Settle on the midpoint of the zone of possible agreement.
///
/// The zone is `[seller_min, min(ask, buyer_max)]`. Its midpoint is rounded
/// half-up to the tick for that midpoint and then clamped back into the zone,
/// so the price can sit off-tick only at one of the two limits.
pub fn settle_with(limits: Limits, ticks: Ticks) -> Result<Settlement, SettleError> {
    let Limits {
        ask,
        seller_min,
        buyer_max,
    } = limits;
    for amount in [ask, seller_min, buyer_max] {
        if amount <= 0 {
            return Err(SettleError::NotPositive);
        }
        if amount > MAX_CENTS {
            return Err(SettleError::TooLarge);
        }
    }
    if seller_min > ask {
        return Err(SettleError::MinAboveAsk);
    }
    ticks.validate()?;

    let ceiling = ask.min(buyer_max);
    if ceiling < seller_min {
        return Ok(Settlement::NoDeal);
    }
    // Work with twice the midpoint so everything stays in whole cents.
    let twice_mid = seller_min + ceiling;
    let tick = if twice_mid < 2 * ticks.small_below {
        ticks.small_tick
    } else {
        ticks.large_tick
    };
    // round_half_up((twice_mid / 2) / tick) * tick
    let rounded = (twice_mid + tick).div_euclid(2 * tick) * tick;
    let price = rounded.clamp(seller_min, ceiling);
    Ok(Settlement::Deal {
        price,
        saved_vs_ask: ask - price,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn price(ask: Cents, seller_min: Cents, buyer_max: Cents) -> Option<Cents> {
        match settle(Limits {
            ask,
            seller_min,
            buyer_max,
        })
        .unwrap()
        {
            Settlement::Deal { price, .. } => Some(price),
            Settlement::NoDeal => None,
        }
    }

    #[test]
    fn demo_scenario() {
        assert_eq!(price(2000, 1500, 1800), Some(1700));
    }

    #[test]
    fn budget_above_ask_still_settles_below_ask() {
        assert_eq!(price(2000, 1500, 3000), Some(1800));
    }

    #[test]
    fn small_items_use_half_dollar_ticks() {
        assert_eq!(price(200, 100, 300), Some(150));
    }

    #[test]
    fn exact_overlap_settles_at_the_single_price() {
        assert_eq!(price(1000, 800, 800), Some(800));
    }

    #[test]
    fn no_overlap_is_no_deal() {
        assert_eq!(price(3000, 2500, 2000), None);
    }

    #[test]
    fn off_tick_limits_clamp_into_the_zone() {
        // Zone [10.75, 10.80]: midpoint rounds to $11, clamped to $10.80.
        assert_eq!(price(1080, 1075, 2000), Some(1080));
    }

    #[test]
    fn rejects_bad_limits() {
        let l = |ask, seller_min, buyer_max| {
            settle(Limits {
                ask,
                seller_min,
                buyer_max,
            })
        };
        assert_eq!(l(0, 0, 100), Err(SettleError::NotPositive));
        assert_eq!(l(100, 200, 300), Err(SettleError::MinAboveAsk));
        assert_eq!(l(MAX_CENTS + 1, 1, 1), Err(SettleError::TooLarge));
        let odd = Ticks {
            small_below: 1030,
            small_tick: 50,
            large_tick: 100,
        };
        assert_eq!(
            settle_with(
                Limits {
                    ask: 100,
                    seller_min: 50,
                    buyer_max: 100
                },
                odd
            ),
            Err(SettleError::BadTicks)
        );
    }

    #[test]
    fn crossing_the_tick_threshold_never_lowers_the_price() {
        // With the tick chosen from the ceiling, $9.99 → $10.00 here dropped $7.50 to $7.00.
        assert_eq!(price(5000, 451, 999), Some(750));
        assert_eq!(price(5000, 451, 1000), Some(750));
    }

    #[test]
    fn largest_amounts_do_not_overflow() {
        assert_eq!(
            price(MAX_CENTS, MAX_CENTS - 100, MAX_CENTS),
            Some(MAX_CENTS)
        );
    }

    #[test]
    fn refusal_carries_no_data() {
        // A unit variant: nothing about the limits can ride along with a refusal.
        assert!(matches!(Settlement::NoDeal, Settlement::NoDeal));
        assert_eq!(format!("{:?}", Settlement::NoDeal), "NoDeal");
    }
}
