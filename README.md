# pairing-core

Price settlement for two-sided marketplaces where software agents negotiate on
behalf of people.

Each person delegates a private limit once: the seller a minimum, the buyer a
maximum. When a buyer and a seller are paired, `pairing-core` settles on a price
both have already authorised, and neither limit ever reaches the other side.
It is written in Rust, has no dependencies, and also ships as a ~1 KB
WebAssembly module for JavaScript.

```rust
use pairing_core::{settle, Limits, Settlement};

// Ask $20, seller accepts down to $15, buyer pays up to $18 → $17.
let s = settle(Limits { ask: 2000, seller_min: 1500, buyer_max: 1800 })?;
assert_eq!(s, Settlement::Deal { price: 1700, saved_vs_ask: 300 });

// No overlap: the refusal carries no data at all.
let s = settle(Limits { ask: 3000, seller_min: 2500, buyer_max: 2000 })?;
assert_eq!(s, Settlement::NoDeal);
```

```js
import { loadSync } from "pairing-core";

const core = loadSync(); // or: await load() in browsers
core.settle({ ask: 2000, sellerMin: 1500, buyerMax: 1800 }); // { ok: true, price: 1700, savedVsAsk: 300 }
core.settle({ ask: 3000, sellerMin: 2500, buyerMax: 2000 }); // { ok: false }
```

## Why it matters

Voice assistants and AI agents are starting to buy and sell for people. An
agent that negotiates needs to know how far its person will go, and that number
is exactly what the other side must never learn. Letting a language model pick
the price makes it hard to audit and easy to talk into revealing a limit.

`pairing-core` keeps the price decision out of the model: the agent collects
the limits, this library settles, and the agent only phrases the result. The
outcome is deterministic, explainable in one sentence, and its privacy
properties hold by construction.

## Guarantees

| Guarantee | How it is enforced |
|---|---|
| The price is inside both limits and at most the asking price | Exhaustive test over a dense grid of amounts |
| A deal exists exactly when the limits overlap | Exhaustive test |
| A refusal reveals nothing about the limits | `Settlement::NoDeal` has no fields; the JS wrapper returns `{ ok: false }` |
| Raising the ask, the seller's minimum or the buyer's maximum never lowers the price | Exhaustive test across neighbouring inputs |
| The price is the midpoint of the zone of agreement, rounded to a price tick | Exhaustive test: within half a tick of the midpoint unless a limit binds |
| Exact money | Integer minor units (cents) everywhere; the WASM boundary rejects fractional cents instead of rounding them |

The monotonicity check earned its place: it caught a defect in the TypeScript
rule this library replaced, where a buyer raising their maximum from $9.99 to
$10.00 could lower the price from $7.50 to $7.00.

## How the price is chosen

1. The zone of possible agreement is `[seller_min, min(ask, buyer_max)]`. If it
   is empty, the result is `NoDeal`.
2. Take the zone's midpoint and round it half-up to the price tick for that
   midpoint. The default `Ticks::USD` uses $0.50 steps below $10 and whole
   dollars from $10 up; `Ticks::CENT` and custom schedules are available.
3. Clamp the result into the zone, so the price only sits off-tick when one of
   the two limits binds.

A custom schedule's threshold must lie on both tick grids; `Ticks::validate`
checks this, and it is what keeps the price monotone across the switch.

## WebAssembly interface

The module has no imports and exports plain functions, so any runtime with
`WebAssembly` can call it without generated glue code. Amounts are `f64` values
holding whole cents.

| Export | Returns |
|---|---|
| `pc_abi_version()` | `1` |
| `pc_settle(ask, seller_min, buyer_max)` | price in cents, `-1` no deal, `-2` invalid amount, `-3` minimum above ask |
| `pc_settle_with(ask, seller_min, buyer_max, small_below, small_tick, large_tick)` | same, with a custom tick schedule (`-2` for an invalid schedule) |

## Build and test

```bash
cargo test                                          # unit, exhaustive invariant and vector tests
rustup target add wasm32-unknown-unknown
cd js && npm run build && npm test                  # builds pairing_core.wasm, runs the same vectors in JS
```

[`tests/vectors.csv`](tests/vectors.csv) lists hand-checked inputs and prices.
Any other implementation can run them to prove it settles identically.

## Used by

[VoiceMarket](https://github.com/adamchoi787/voice-market-mcp), a voice-first
marketplace for Alexa+ that pairs buyers and sellers directly. Its MCP server
settles every deal through this module.

## License

MIT
