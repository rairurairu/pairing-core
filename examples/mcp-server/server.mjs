// An MCP server where the agent never sees or decides a price. The seller's
// minimum lives only on the server, the buyer's budget goes in once, and the
// tool result carries the settlement and nothing else.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { loadSync } from "pairing-core";
import { z } from "zod";

const core = loadSync();
const dollars = (cents) => `$${(cents / 100).toFixed(2)}`;

/** Listings: the ask is public, sellerMin is private and never leaves this file. */
export function createServer(listings) {
  const floors = new Map(); // `${listingId}:${buyerId}` -> best price this buyer was already offered

  const server = new McpServer({ name: "pairing-core-example", version: "0.2.0" });

  server.registerTool(
    "list_listings",
    { description: "List items for sale with their public asking prices." },
    async () => ({
      content: [{ type: "text", text: listings.map((l) => `${l.id}: ${l.title}, ${dollars(l.ask)}`).join("\n") }],
    }),
  );

  server.registerTool(
    "make_offer",
    {
      description:
        "Settle a price for one listing within the buyer's budget. The result is final: either a price at or below " +
        "the budget, or no deal. Do not retry with a lower budget; it cannot settle lower.",
      inputSchema: {
        listing_id: z.string(),
        buyer_id: z.string(),
        budget_cents: z.number().int().positive().describe("The most the buyer will pay, in whole cents."),
      },
    },
    async ({ listing_id, buyer_id, budget_cents }) => {
      const listing = listings.find((l) => l.id === listing_id);
      if (!listing) return { isError: true, content: [{ type: "text", text: `No listing ${listing_id}.` }] };

      const key = `${listing_id}:${buyer_id}`;
      const floor = floors.get(key) ?? 0;
      const s = core.settle({ ask: listing.ask, sellerMin: listing.sellerMin, buyerMax: budget_cents, floor });

      if (s.ok) floors.set(key, Math.max(floor, s.price));
      else if (budget_cents < listing.ask) floors.set(key, listing.ask);

      const text = s.ok
        ? `Deal: ${listing.title} for ${dollars(s.price)} (${dollars(s.savedVsAsk)} below asking).`
        : `No deal for ${listing.title}.`;
      return { content: [{ type: "text", text }], structuredContent: s };
    },
  );

  return server;
}

export const demoListings = [
  { id: "lamp", title: "Desk lamp", ask: 2000, sellerMin: 1500 },
  { id: "bike", title: "Road bike", ask: 32000, sellerMin: 27500 },
];
