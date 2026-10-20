//! A buyer's agent tries to find the seller's minimum by walking away and
//! coming back with lower budgets. The caller keeps one number per buyer and
//! listing, the floor, and the probe learns nothing.
//!
//! Run with `cargo run --example walk_away`.

use pairing_core::{settle, Cents, Limits, Settlement};

/// What a marketplace stores for one buyer and one listing.
struct Pairing {
    ask: Cents,
    seller_min: Cents,
    floor: Cents,
}

impl Pairing {
    fn offer(&mut self, buyer_max: Cents) -> Settlement {
        let s = settle(Limits {
            ask: self.ask,
            seller_min: self.seller_min,
            buyer_max,
            floor: self.floor,
        })
        .expect("valid amounts");
        match s {
            // The buyer was offered this price; if they walk away, it becomes their floor.
            Settlement::Deal { price, .. } => self.floor = self.floor.max(price),
            // An offer below the ask that the seller refused: from now on only the ask settles.
            Settlement::NoDeal if buyer_max < self.ask => self.floor = self.ask,
            Settlement::NoDeal => {}
        }
        s
    }
}

fn main() {
    // Ask $20.00; the seller privately accepts down to $15.00.
    let mut p = Pairing {
        ask: 2000,
        seller_min: 1500,
        floor: 0,
    };

    for budget in [1900, 1700, 1600, 1500, 1400] {
        let result = match p.offer(budget) {
            Settlement::Deal { price, .. } => format!("deal at {price} (buyer walks away)"),
            Settlement::NoDeal => "no deal".to_string(),
        };
        println!("budget {budget:>4} -> {result}");
    }
    // Only the first offer settles. Every lower budget gets the same "no deal",
    // so the probe never learns that 1500 would have been accepted.
    assert_eq!(p.offer(1500), Settlement::NoDeal);
    assert_eq!(
        p.offer(2000),
        Settlement::Deal {
            price: 2000,
            saved_vs_ask: 0
        }
    );
}
