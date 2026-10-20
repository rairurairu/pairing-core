import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { load, loadSync } from "./index.js";

// The compiled module must reproduce every published vector, exactly as the Rust tests do.

const core = loadSync();
const vectors = readFileSync(new URL("../tests/vectors.csv", import.meta.url), "utf8")
  .split("\n").slice(1).map((l) => l.split("#")[0].trim()).filter(Boolean)
  .map((l) => l.split(",").map((f) => f.trim()));

test("published vectors match", () => {
  assert.ok(vectors.length >= 15);
  for (const [ask, min, max, floor, price] of vectors) {
    const r = core.settle({ ask: +ask, sellerMin: +min, buyerMax: +max, floor: +floor });
    if (price === "none") assert.deepEqual(r, { ok: false }, `${ask},${min},${max},${floor}`);
    else assert.deepEqual(r, { ok: true, price: +price, savedVsAsk: +ask - +price }, `${ask},${min},${max},${floor}`);
  }
});

test("floor defaults to none", () => {
  assert.deepEqual(core.settle({ ask: 2000, sellerMin: 1500, buyerMax: 1800 }), { ok: true, price: 1800, savedVsAsk: 200 });
});

test("bad input is rejected, never rounded", () => {
  for (const bad of [
    { ask: 0, sellerMin: 1, buyerMax: 1 },
    { ask: 10.5, sellerMin: 5, buyerMax: 10 },
    { ask: NaN, sellerMin: 1, buyerMax: 1 },
    { ask: 1e18, sellerMin: 1, buyerMax: 1 },
    { ask: 100, sellerMin: 50, buyerMax: 100, floor: -1 },
    { ask: 100, sellerMin: 50, buyerMax: 100, floor: 0.5 },
  ]) {
    assert.throws(() => core.settle(bad), RangeError);
  }
  assert.throws(() => core.settle({ ask: 100, sellerMin: 200, buyerMax: 300 }), /minimum is above/);
});

test("async load works from bytes", async () => {
  const bytes = readFileSync(new URL("./pairing_core.wasm", import.meta.url));
  const fromBytes = await load(bytes);
  assert.deepEqual(fromBytes.settle({ ask: 200, sellerMin: 100, buyerMax: 150 }), { ok: true, price: 150, savedVsAsk: 50 });
});
