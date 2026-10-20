//! Every documented guarantee, checked exhaustively over a dense grid of
//! amounts plus hand-picked boundary values. Runs in a few seconds in debug builds.

use pairing_core::{settle, Cents, Limits, Settlement};

fn amounts() -> Vec<Cents> {
    let mut v: Vec<Cents> = (1..=1300).step_by(13).collect();
    v.extend([
        1, 25, 49, 50, 51, 99, 100, 101, 149, 150, 151, 950, 975, 999, 1000, 1001, 1049, 1050,
        1075, 1080, 1500, 1625, 1800, 1999, 2000, 2050, 9_999, 10_000, 123_457,
    ]);
    v.sort_unstable();
    v.dedup();
    v
}

fn lim(ask: Cents, seller_min: Cents, buyer_max: Cents, floor: Cents) -> Limits {
    Limits {
        ask,
        seller_min,
        buyer_max,
        floor,
    }
}

fn price(l: Limits) -> Option<Cents> {
    match settle(l).expect("valid limits") {
        Settlement::Deal { price, .. } => Some(price),
        Settlement::NoDeal => None,
    }
}

/// Every valid (ask, seller_min, buyer_max) on the grid, with no floor.
fn each_valid(mut f: impl FnMut(Limits, Settlement)) {
    let xs = amounts();
    for &ask in &xs {
        for &seller_min in xs.iter().filter(|&&m| m <= ask) {
            for &buyer_max in &xs {
                let l = lim(ask, seller_min, buyer_max, 0);
                f(l, settle(l).expect("valid limits"));
            }
        }
    }
}

#[test]
fn deal_exactly_when_the_limits_overlap() {
    each_valid(|l, s| {
        let overlap = l.seller_min <= l.ask.min(l.buyer_max);
        assert_eq!(matches!(s, Settlement::Deal { .. }), overlap, "{l:?}");
    });
}

#[test]
fn price_stays_inside_both_limits_and_at_most_the_ask() {
    each_valid(|l, s| {
        if let Settlement::Deal {
            price,
            saved_vs_ask,
        } = s
        {
            assert!(l.seller_min <= price, "{l:?} → {price}");
            assert!(price <= l.buyer_max, "{l:?} → {price}");
            assert!(price <= l.ask, "{l:?} → {price}");
            assert_eq!(saved_vs_ask, l.ask - price);
        }
    });
}

#[test]
fn the_sellers_minimum_never_moves_the_price() {
    // Non-interference: every minimum that allows the deal gives the same price,
    // so the price carries no information about which minimum it was.
    let xs = amounts();
    for &ask in &xs {
        for &buyer_max in &xs {
            let prices: Vec<Cents> = xs
                .iter()
                .filter(|&&m| m <= ask)
                .filter_map(|&m| price(lim(ask, m, buyer_max, 0)))
                .collect();
            assert!(
                prices.windows(2).all(|w| w[0] == w[1]),
                "ask {ask}, max {buyer_max}: price depends on the minimum: {prices:?}"
            );
        }
    }
}

#[test]
fn a_budget_at_or_above_the_ask_never_moves_the_price() {
    each_valid(|l, s| {
        if l.buyer_max >= l.ask {
            if let Settlement::Deal { price, .. } = s {
                assert_eq!(price, l.ask, "{l:?}");
            }
        }
    });
}

#[test]
fn walking_away_never_pays() {
    // After an offer at `floor`, no later budget gets a lower price, however the
    // buyer varies it. So a step-by-step lower budget cannot find the minimum.
    let xs = amounts();
    for &ask in xs.iter().step_by(3) {
        for &seller_min in xs.iter().filter(|&&m| m <= ask).step_by(3) {
            for &first in &xs {
                let Some(offered) = price(lim(ask, seller_min, first, 0)) else {
                    continue;
                };
                for &again in &xs {
                    if let Some(p) = price(lim(ask, seller_min, again, offered)) {
                        assert!(
                            p >= offered,
                            "ask {ask} min {seller_min}: {offered} then {p}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn raising_the_ask_or_the_budget_never_lowers_the_price() {
    let xs = amounts();
    for w in xs.windows(2) {
        let (lo, hi) = (w[0], w[1]);
        for &a in &xs {
            for &b in &xs {
                // buyer_max: lo → hi
                if let (Some(x), Some(y)) = (
                    price(lim(a, b.min(a), lo, 0)),
                    price(lim(a, b.min(a), hi, 0)),
                ) {
                    assert!(y >= x, "buyer_max {lo}→{hi} lowered the price");
                }
                // ask: lo → hi
                if b <= lo {
                    if let (Some(x), Some(y)) = (price(lim(lo, b, a, 0)), price(lim(hi, b, a, 0))) {
                        assert!(y >= x, "ask {lo}→{hi} lowered the price");
                    }
                }
            }
        }
    }
}

#[test]
fn published_vectors_match() {
    let mut checked = 0;
    for line in include_str!("vectors.csv").lines().skip(1) {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        let n = |i: usize| f[i].parse::<Cents>().unwrap();
        let expected = match f[4] {
            "none" => Settlement::NoDeal,
            p => Settlement::Deal {
                price: p.parse().unwrap(),
                saved_vs_ask: n(0) - p.parse::<Cents>().unwrap(),
            },
        };
        assert_eq!(
            settle(lim(n(0), n(1), n(2), n(3))).unwrap(),
            expected,
            "vector {line}"
        );
        checked += 1;
    }
    assert!(checked >= 15);
}
