#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const examplesDir = path.join(root, "examples");
const DEFAULT_EXAMPLE = "triangle";

function listExamples() {
  return fs
    .readdirSync(examplesDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && entry.name !== "shared")
    .map((entry) => entry.name)
    .filter((name) => fs.existsSync(path.join(examplesDir, name, "package.json")))
    .sort();
}

function parseArgs(argv) {
  const args = argv.slice(2);
  let watchLibrary = false;
  let name = DEFAULT_EXAMPLE;

  for (const arg of args) {
    if (arg === "--all") {
      watchLibrary = true;
    } else if (!arg.startsWith("-")) {
      name = arg;
    }
  }

  return { name, watchLibrary };
}

function examplePackageName(name) {
  const pkgPath = path.join(examplesDir, name, "package.json");
  if (!fs.existsSync(pkgPath)) {
    return null;
  }

  try {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
    return typeof pkg.name === "string" ? pkg.name : null;
  } catch {
    return null;
  }
}

const { name, watchLibrary } = parseArgs(process.argv);
const packageName = examplePackageName(name);

if (!packageName) {
  console.error(`Unknown example: ${name}`);
  console.error(`Available: ${listExamples().join(", ")}`);
  process.exit(1);
}

const pnpmArgs = watchLibrary
  ? ["--parallel", "--filter", "belfast", "--filter", packageName, "dev"]
  : ["--filter", packageName, "dev"];

const result = spawnSync("pnpm", pnpmArgs, {
  cwd: root,
  stdio: "inherit",
  shell: process.platform === "win32",
});

process.exit(result.status ?? 1);
