import assert from "node:assert/strict";
import { test } from "node:test";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer, demoListings } from "./server.mjs";

async function connect() {
  const [a, b] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: "test", version: "0" });
  await Promise.all([createServer(demoListings).connect(a), client.connect(b)]);
  return (args) => client.callTool({ name: "make_offer", arguments: args });
}

test("a budget below the ask settles as the buyer's offer", async () => {
  const offer = await connect();
  const r = await offer({ listing_id: "lamp", buyer_id: "ana", budget_cents: 1800 });
  assert.deepEqual(r.structuredContent, { ok: true, price: 1800, savedVsAsk: 200 });
  assert.match(r.content[0].text, /\$18\.00/);
});

test("nothing the agent sees mentions the seller's minimum", async () => {
  const offer = await connect();
  for (const budget_cents of [1000, 1499, 1800, 2500]) {
    const r = await offer({ listing_id: "lamp", buyer_id: `b${budget_cents}`, budget_cents });
    assert.doesNotMatch(JSON.stringify(r), /1500|15\.00/);
  }
});

test("walking away and coming back lower never settles lower", async () => {
  const offer = await connect();
  const first = await offer({ listing_id: "lamp", buyer_id: "ben", budget_cents: 1900 });
  assert.equal(first.structuredContent.price, 1900);
  for (const budget_cents of [1700, 1600, 1500]) {
    const r = await offer({ listing_id: "lamp", buyer_id: "ben", budget_cents });
    assert.deepEqual(r.structuredContent, { ok: false });
  }
});

test("a refusal is the same whatever the gap", async () => {
  const offer = await connect();
  const near = await offer({ listing_id: "lamp", buyer_id: "c1", budget_cents: 1499 });
  const far = await offer({ listing_id: "lamp", buyer_id: "c2", budget_cents: 100 });
  assert.deepEqual(near.structuredContent, far.structuredContent);
  assert.equal(near.content[0].text, far.content[0].text);
});
