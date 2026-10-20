# pairing-core

[![CI](https://github.com/rairurairu/pairing-core/actions/workflows/ci.yml/badge.svg)](https://github.com/rairurairu/pairing-core/actions/workflows/ci.yml)

Price settlement for two-sided marketplaces where software agents trade on
behalf of people.

Each person delegates a private limit once: the seller a minimum, the buyer a
maximum. When a buyer and a seller are paired, `pairing-core` settles on a price
both have already authorised. The seller's minimum never shows through the
price, and the buyer's budget shows only when it is below the ask, as the
buyer's offer. It is written in Rust,
has no dependencies, and also ships as a ~1 KB WebAssembly module for JavaScript.

```rust
use pairing_core::{settle, Limits, Settlement};

// Ask $20, seller accepts down to $15, buyer pays up to $18 → $18, the buyer's offer.
let s = settle(Limits { ask: 2000, seller_min: 1500, buyer_max: 1800, floor: 0 })?;
assert_eq!(s, Settlement::Deal { price: 1800, saved_vs_ask: 200 });

// No overlap: the refusal carries no data at all.
let s = settle(Limits { ask: 3000, seller_min: 2500, buyer_max: 2000, floor: 0 })?;
assert_eq!(s, Settlement::NoDeal);
```

```js
import { loadSync } from "pairing-core";

const core = loadSync(); // or: await load() in browsers
core.settle({ ask: 2000, sellerMin: 1500, buyerMax: 1800 }); // { ok: true, price: 1800, savedVsAsk: 200 }
core.settle({ ask: 3000, sellerMin: 2500, buyerMax: 2000 }); // { ok: false }
```

## Why it matters

Voice assistants and AI agents are starting to buy and sell for people. An
agent acting for someone needs to know how far its person will go, and that
number is exactly what the other side must never learn. Two agents haggling
turn by turn cost model calls and time, are hard to audit, and can be talked
into revealing a limit.

`pairing-core` replaces the haggling with one deterministic rule: the agent
collects the limits, this library settles in microseconds, and the agent only
phrases the result.

## The rule

The asking price is public. A budget below it is the buyer's offer. The
seller's private minimum decides only whether that offer is accepted:

```text
price = min(ask, buyer_max)      deal  iff  price >= seller_min  and  price >= floor
```

This is the "best offer with automatic acceptance" pattern people already know
from online marketplaces. A rule that splits the difference between the two
limits looks fairer, but its price is a function of both limits, so each side
can work out the other's limit from the price: with the midpoint, a buyer who
budgets the full ask learns the seller's minimum as `2 × price − ask`.

`floor` closes the remaining gap. It is the best price this buyer was already
offered for this listing and walked away from, and no later settlement for that
pair goes below it. Without it, a buyer could decline, ask again with a lower
budget, and step down until the seller's minimum stops them.

## Guarantees

| Guarantee | How it is enforced |
|---|---|
| The price is inside both limits and at most the asking price | Exhaustive test over a dense grid of amounts |
| A deal exists exactly when the limits overlap | Exhaustive test |
| The seller's minimum never moves the price, so a deal reveals only that the minimum is at most the price | Exhaustive non-interference test: every minimum that allows a deal gives the same price |
| A budget at or above the ask never moves the price | Exhaustive test |
| Walking away never pays: with the floor set, no later price for the same buyer and listing is lower | Exhaustive test over every first offer and every later budget |
| A refusal reveals nothing about the limits | `Settlement::NoDeal` has no fields; the JS wrapper returns `{ ok: false }` |
| Raising the ask or the buyer's maximum never lowers the price | Exhaustive test across neighbouring inputs |
| Exact money | Integer minor units (cents) everywhere; the WASM boundary rejects fractional cents instead of rounding them |

## WebAssembly interface

The module has no imports and exports plain functions, so any runtime with
`WebAssembly` can call it without generated glue code. Amounts are `f64` values
holding whole cents.

| Export | Returns |
|---|---|
| `pc_abi_version()` | `2` |
| `pc_settle(ask, seller_min, buyer_max, floor)` | price in cents, `-1` no deal, `-2` invalid amount, `-3` minimum above ask |

## Build and test

```bash
cargo test                                          # unit, exhaustive invariant and vector tests
rustup target add wasm32-unknown-unknown
cd js && npm run build && npm test                  # builds pairing_core.wasm, runs the same vectors in JS
```

[`tests/vectors.csv`](tests/vectors.csv) lists hand-checked inputs and prices.
Any other implementation can run them to prove it settles identically.

## Examples

- [`examples/mcp-server`](examples/mcp-server): an MCP server with a
  `make_offer` tool. The seller's minimum stays on the server, the agent passes
  the buyer's budget once, and the tool result carries only the settlement. Its
  tests check that no result mentions the minimum and that coming back with a
  lower budget never settles lower. Run it with `npm install && npm start`.
- [`examples/walk_away.rs`](examples/walk_away.rs): a buyer probes downward
  after walking away, and the floor turns every probe into the same "no deal".
  Run it with `cargo run --example walk_away`.

## Used by

[AmazonPairUp](https://github.com/adamchoi787/voice-market-mcp), a voice-first
marketplace for Alexa+ that pairs buyers and sellers directly. Its MCP server
settles every deal through this module.

## License

MIT
