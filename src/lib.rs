//! # pairing-core
//!
//! Price settlement for two-sided marketplaces where software agents trade on
//! behalf of people. Each side delegates a private limit once: the seller a
//! minimum, the buyer a maximum. When the two are paired, `pairing-core`
//! settles on a price both people have already authorised. The seller's
//! minimum never shows through the price, and the buyer's budget shows only
//! when it is below the ask, as the buyer's offer.
//!
//! ```
//! use pairing_core::{settle, Limits, Settlement};
//!
//! // Ask $20, seller accepts down to $15, buyer pays up to $18 → $18, the buyer's offer.
//! let s = settle(Limits { ask: 2000, seller_min: 1500, buyer_max: 1800, floor: 0 }).unwrap();
//! assert_eq!(s, Settlement::Deal { price: 1800, saved_vs_ask: 200 });
//!
//! // A budget at or above the ask pays the ask.
//! let s = settle(Limits { ask: 2000, seller_min: 1500, buyer_max: 3000, floor: 0 }).unwrap();
//! assert_eq!(s, Settlement::Deal { price: 2000, saved_vs_ask: 0 });
//!
//! // No overlap: the refusal carries no data at all.
//! let s = settle(Limits { ask: 3000, seller_min: 2500, buyer_max: 2000, floor: 0 }).unwrap();
//! assert_eq!(s, Settlement::NoDeal);
//! ```
//!
//! ## The rule
//!
//! The asking price is public; a budget below it is the buyer's offer; the
//! seller's minimum decides only whether that offer is accepted:
//!
//! ```text
//! price = min(ask, buyer_max)      deal  iff  price >= seller_min  and  price >= floor
//! ```
//!
//! `floor` is the best price this buyer was already offered for this listing
//! and walked away from (0 when there is none), so trying again with a lower
//! budget never pays.
//!
//! ## Guarantees
//!
//! - **Inside both limits:** `seller_min <= price <= min(ask, buyer_max)`.
//! - **The seller's minimum never moves the price.** Every minimum that allows
//!   the deal gives the same price, so a buyer learns only that the minimum is
//!   at most what they pay.
//! - **A budget at or above the ask never moves the price.** The seller learns
//!   only that the budget covers the ask.
//! - **Walking away never pays:** with the floor set, no later settlement for
//!   the same buyer and listing is below an offer already made, so lowering the
//!   budget step by step cannot find the seller's minimum.
//! - **Refusals reveal nothing:** [`Settlement::NoDeal`] has no fields.
//! - **Monotone:** raising the ask or the buyer's maximum never lowers the price.
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

/// Both sides' delegated limits for one pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The seller's public asking price.
    pub ask: Cents,
    /// The seller's private minimum. Never shown to the buyer.
    pub seller_min: Cents,
    /// The buyer's private maximum. Shown to the seller only as the price,
    /// when it is below the ask.
    pub buyer_max: Cents,
    /// The best price this buyer was already offered for this listing and
    /// walked away from, or 0 for none.
    pub floor: Cents,
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
    /// An amount was zero or negative (the floor may be zero).
    NotPositive,
    /// An amount was above [`MAX_CENTS`].
    TooLarge,
    /// The seller's minimum was above their own asking price.
    MinAboveAsk,
}

impl core::fmt::Display for SettleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NotPositive => "amounts must be positive",
            Self::TooLarge => "amount is too large",
            Self::MinAboveAsk => "the seller's minimum is above the asking price",
        })
    }
}

impl std::error::Error for SettleError {}

/// Settle one pairing: the ask, or the buyer's lower budget as an offer that
/// the seller's minimum accepts or refuses.
pub fn settle(limits: Limits) -> Result<Settlement, SettleError> {
    let Limits {
        ask,
        seller_min,
        buyer_max,
        floor,
    } = limits;
    for amount in [ask, seller_min, buyer_max] {
        if amount <= 0 {
            return Err(SettleError::NotPositive);
        }
    }
    if floor < 0 {
        return Err(SettleError::NotPositive);
    }
    if [ask, seller_min, buyer_max, floor]
        .iter()
        .any(|&a| a > MAX_CENTS)
    {
        return Err(SettleError::TooLarge);
    }
    if seller_min > ask {
        return Err(SettleError::MinAboveAsk);
    }

    let price = ask.min(buyer_max);
    if price < seller_min || price < floor {
        return Ok(Settlement::NoDeal);
    }
    Ok(Settlement::Deal {
        price,
        saved_vs_ask: ask - price,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn price(ask: Cents, seller_min: Cents, buyer_max: Cents, floor: Cents) -> Option<Cents> {
        match settle(Limits {
            ask,
            seller_min,
            buyer_max,
            floor,
        })
        .unwrap()
        {
            Settlement::Deal { price, .. } => Some(price),
            Settlement::NoDeal => None,
        }
    }

    #[test]
    fn demo_scenario_pays_the_buyers_offer() {
        assert_eq!(price(2000, 1500, 1800, 0), Some(1800));
    }

    #[test]
    fn budget_above_ask_pays_the_ask() {
        assert_eq!(price(2000, 1500, 3000, 0), Some(2000));
    }

    #[test]
    fn exact_overlap_settles_at_the_single_price() {
        assert_eq!(price(1000, 800, 800, 0), Some(800));
    }

    #[test]
    fn no_overlap_is_no_deal() {
        assert_eq!(price(3000, 2500, 2000, 0), None);
    }

    #[test]
    fn coming_back_lower_after_walking_away_is_no_deal() {
        // Offered $20 and declined; a $16 budget would clear the $15 minimum but not the floor.
        assert_eq!(price(2000, 1500, 1600, 2000), None);
        // Coming back at the same price is fine.
        assert_eq!(price(2000, 1500, 2500, 2000), Some(2000));
    }

    #[test]
    fn rejects_bad_limits() {
        let l = |ask, seller_min, buyer_max, floor| {
            settle(Limits {
                ask,
                seller_min,
                buyer_max,
                floor,
            })
        };
        assert_eq!(l(0, 0, 100, 0), Err(SettleError::NotPositive));
        assert_eq!(l(100, 50, 100, -1), Err(SettleError::NotPositive));
        assert_eq!(l(100, 200, 300, 0), Err(SettleError::MinAboveAsk));
        assert_eq!(l(MAX_CENTS + 1, 1, 1, 0), Err(SettleError::TooLarge));
    }

    #[test]
    fn largest_amounts_do_not_overflow() {
        assert_eq!(
            price(MAX_CENTS, MAX_CENTS - 100, MAX_CENTS, MAX_CENTS),
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
