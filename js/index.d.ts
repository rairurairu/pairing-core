/** Amounts are whole cents. */
export interface Limits {
  /** The seller's public asking price. */
  ask: number;
  /** The seller's private minimum. */
  sellerMin: number;
  /** The buyer's private maximum. */
  buyerMax: number;
}

/** Prices below `smallBelow` (by zone midpoint) use `smallTick`, others `largeTick`. */
export interface Ticks {
  smallBelow: number;
  smallTick: number;
  largeTick: number;
}

/** A refusal carries no data, so it cannot leak how far apart the limits were. */
export type Settlement = { ok: true; price: number; savedVsAsk: number } | { ok: false };

export interface PairingCore {
  /** Throws RangeError for non-positive, fractional or out-of-range amounts, or min above ask. */
  settle(limits: Limits, ticks?: Ticks): Settlement;
}

export const ABI_VERSION: 1;
export const USD_TICKS: Readonly<Ticks>;
export function loadSync(): PairingCore;
export function load(source?: BufferSource | Response | Promise<Response> | URL | string): Promise<PairingCore>;
