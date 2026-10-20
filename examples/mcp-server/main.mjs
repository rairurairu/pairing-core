// Run over stdio, e.g. as a local MCP server in Claude Desktop or Claude Code.
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createServer, demoListings } from "./server.mjs";

await createServer(demoListings).connect(new StdioServerTransport());
