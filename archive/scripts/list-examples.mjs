#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const examplesDir = path.join(path.resolve(__dirname, ".."), "examples");

const examples = fs
  .readdirSync(examplesDir, { withFileTypes: true })
  .filter((entry) => entry.isDirectory() && entry.name !== "shared")
  .map((entry) => entry.name)
  .filter((name) => fs.existsSync(path.join(examplesDir, name, "package.json")))
  .sort();

if (examples.length === 0) {
  console.log("No examples found.");
  process.exit(0);
}

console.log("Examples:");
for (const name of examples) {
  console.log(`  ${name}`);
}

console.log("\nRun:");
console.log("  pnpm dev:example <name>");
console.log("  pnpm dev:all <name>");
