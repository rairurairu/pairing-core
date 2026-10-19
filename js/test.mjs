import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { load, loadSync, USD_TICKS } from "./index.js";

// The compiled module must reproduce every published vector, exactly as the Rust tests do.

const core = loadSync();
const vectors = readFileSync(new URL("../tests/vectors.csv", import.meta.url), "utf8")
  .split("\n").slice(1).map((l) => l.split("#")[0].trim()).filter(Boolean)
  .map((l) => l.split(",").map((f) => f.trim()));

test("published vectors match", () => {
  assert.ok(vectors.length >= 20);
  for (const [ask, min, max, price] of vectors) {
    const r = core.settle({ ask: +ask, sellerMin: +min, buyerMax: +max });
    if (price === "none") assert.deepEqual(r, { ok: false }, `${ask},${min},${max}`);
    else assert.deepEqual(r, { ok: true, price: +price, savedVsAsk: +ask - +price }, `${ask},${min},${max}`);
  }
});

test("explicit USD ticks give the same answers", () => {
  assert.deepEqual(core.settle({ ask: 2000, sellerMin: 1500, buyerMax: 1800 }, USD_TICKS), { ok: true, price: 1700, savedVsAsk: 300 });
});

test("bad input is rejected, never rounded", () => {
  for (const bad of [{ ask: 0, sellerMin: 1, buyerMax: 1 }, { ask: 10.5, sellerMin: 5, buyerMax: 10 }, { ask: NaN, sellerMin: 1, buyerMax: 1 }, { ask: 1e18, sellerMin: 1, buyerMax: 1 }]) {
    assert.throws(() => core.settle(bad), RangeError);
  }
  assert.throws(() => core.settle({ ask: 100, sellerMin: 200, buyerMax: 300 }), /minimum is above/);
  assert.throws(() => core.settle({ ask: 100, sellerMin: 50, buyerMax: 100 }, { smallBelow: 1030, smallTick: 50, largeTick: 100 }), RangeError);
});

test("async load works from bytes", async () => {
  const bytes = readFileSync(new URL("./pairing_core.wasm", import.meta.url));
  const fromBytes = await load(bytes);
  assert.deepEqual(fromBytes.settle({ ask: 200, sellerMin: 100, buyerMax: 300 }), { ok: true, price: 150, savedVsAsk: 50 });
});
