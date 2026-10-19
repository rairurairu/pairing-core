//! Every documented guarantee, checked exhaustively: a dense grid of amounts
//! plus hand-picked boundary values (tick edges, the $10 threshold, off-tick
//! limits). Runs in a few seconds in debug builds.

use pairing_core::{settle, settle_with, Cents, Limits, Settlement, Ticks};

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

fn each_valid(mut f: impl FnMut(Limits, Settlement)) {
    let xs = amounts();
    for &ask in &xs {
        for &seller_min in xs.iter().filter(|&&m| m <= ask) {
            for &buyer_max in &xs {
                let l = Limits {
                    ask,
                    seller_min,
                    buyer_max,
                };
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
fn price_stays_inside_both_limits_and_below_ask() {
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
fn price_is_the_rounded_midpoint_unless_a_limit_binds() {
    each_valid(|l, s| {
        if let Settlement::Deal { price, .. } = s {
            let ceiling = l.ask.min(l.buyer_max);
            let tick = if l.seller_min + ceiling < 2 * Ticks::USD.small_below {
                Ticks::USD.small_tick
            } else {
                Ticks::USD.large_tick
            };
            let at_limit = price == l.seller_min || price == ceiling;
            assert!(
                at_limit || price % tick == 0,
                "{l:?} → {price} is off the {tick} tick"
            );
            // |price − midpoint| ≤ tick / 2, compared in doubled units to stay exact.
            assert!(
                (2 * price - (l.seller_min + ceiling)).abs() <= tick,
                "{l:?} → {price} far from midpoint"
            );
        }
    });
}

#[test]
fn raising_any_limit_never_lowers_the_price() {
    let xs = amounts();
    let p = |ask, seller_min, buyer_max| match settle(Limits {
        ask,
        seller_min,
        buyer_max,
    }) {
        Ok(Settlement::Deal { price, .. }) => Some(price),
        _ => None,
    };
    for w in xs.windows(2) {
        let (lo, hi) = (w[0], w[1]);
        for &a in &xs {
            for &b in &xs {
                // buyer_max: lo → hi
                if let (Some(x), Some(y)) = (p(a, b.min(a), lo), p(a, b.min(a), hi)) {
                    assert!(y >= x, "buyer_max {lo}→{hi} lowered the price");
                }
                // seller_min: lo → hi
                if hi <= a {
                    if let (Some(x), Some(y)) = (p(a, lo, b), p(a, hi, b)) {
                        assert!(y >= x, "seller_min {lo}→{hi} lowered the price");
                    }
                }
                // ask: lo → hi
                if b <= lo {
                    if let (Some(x), Some(y)) = (p(lo, b, a), p(hi, b, a)) {
                        assert!(y >= x, "ask {lo}→{hi} lowered the price");
                    }
                }
            }
        }
    }
}

#[test]
fn cent_ticks_settle_on_the_exact_midpoint() {
    each_valid(|l, _| {
        if let Ok(Settlement::Deal { price, .. }) = settle_with(l, Ticks::CENT) {
            let ceiling = l.ask.min(l.buyer_max);
            assert_eq!(price, (l.seller_min + ceiling + 1) / 2, "{l:?}");
        }
    });
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
        let expected = match f[3] {
            "none" => Settlement::NoDeal,
            p => Settlement::Deal {
                price: p.parse().unwrap(),
                saved_vs_ask: n(0) - p.parse::<Cents>().unwrap(),
            },
        };
        let l = Limits {
            ask: n(0),
            seller_min: n(1),
            buyer_max: n(2),
        };
        assert_eq!(settle(l).unwrap(), expected, "vector {line}");
        checked += 1;
    }
    assert!(checked >= 20);
}
