// pairing-core for JavaScript: the Rust settlement core compiled to a ~1 KB
// WebAssembly module with a plain numeric interface. Works in Node and browsers.

const NO_DEAL = -1;
const MIN_ABOVE_ASK = -3;

/** Numeric interface version this wrapper understands. */
export const ABI_VERSION = 2;

function wrap(instance) {
  const { pc_abi_version, pc_settle } = instance.exports;
  const abi = pc_abi_version();
  if (abi !== ABI_VERSION) throw new Error(`pairing-core: expected ABI ${ABI_VERSION}, module has ${abi}`);

  return Object.freeze({
    /**
     * Settle one pairing: the ask, or the buyer's lower budget as an offer the
     * seller's minimum accepts. `floor` is the best price this buyer already
     * walked away from for this listing (default 0). All amounts are whole cents.
     * Returns { ok: true, price, savedVsAsk } or { ok: false } (no data on refusal).
     */
    settle({ ask, sellerMin, buyerMax, floor = 0 }) {
      const r = pc_settle(ask, sellerMin, buyerMax, floor);
      if (r >= 0) return { ok: true, price: r, savedVsAsk: ask - r };
      if (r === NO_DEAL) return { ok: false };
      if (r === MIN_ABOVE_ASK) throw new RangeError("pairing-core: the seller's minimum is above the asking price");
      throw new RangeError("pairing-core: amounts must be positive whole cents (floor may be 0)");
    },
  });
}

const wasmUrl = () => new URL("./pairing_core.wasm", import.meta.url);

/** Load synchronously from the bundled .wasm file (Node 20.16+ / 22.3+). */
export function loadSync() {
  const fs = globalThis.process?.getBuiltinModule?.("node:fs");
  if (!fs) throw new Error("pairing-core: loadSync needs Node; use load() in browsers");
  return wrap(new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(wasmUrl())), {}));
}

/** Load from the bundled .wasm file, or from bytes / a Response / a URL you provide. */
export async function load(source) {
  if (source === undefined && globalThis.process?.getBuiltinModule) return loadSync();
  let bytes = source ?? wasmUrl();
  if (bytes instanceof URL || typeof bytes === "string") bytes = fetch(bytes);
  if (bytes instanceof Promise) bytes = await bytes;
  if (typeof Response !== "undefined" && bytes instanceof Response) bytes = await bytes.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return wrap(instance);
}
