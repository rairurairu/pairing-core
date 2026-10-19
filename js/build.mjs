// Builds the WebAssembly module and copies it next to the JavaScript wrapper.
import { execFileSync } from "node:child_process";
import { copyFileSync, statSync } from "node:fs";

const root = new URL("../", import.meta.url);
execFileSync("cargo", ["build", "--release", "--target", "wasm32-unknown-unknown"], { cwd: root, stdio: "inherit" });
const from = new URL("target/wasm32-unknown-unknown/release/pairing_core.wasm", root);
const to = new URL("./pairing_core.wasm", import.meta.url);
copyFileSync(from, to);
console.log(`pairing_core.wasm: ${statSync(to).size} bytes`);
