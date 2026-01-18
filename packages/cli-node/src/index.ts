#!/usr/bin/env node
/**
 * opencode-cloud Node.js CLI
 *
 * This is a thin wrapper that calls into the Rust core library via NAPI bindings.
 * The heavy lifting is done in Rust - this just provides npm/npx distribution.
 */

import { getVersionJs } from "@opencode-cloud/core";

const args = process.argv.slice(2);

function printHelp(): void {
  console.log(`
opencode-cloud - Manage your opencode cloud service

USAGE:
    opencode-cloud [OPTIONS] [COMMAND]

OPTIONS:
    -V, --version    Print version information
    -h, --help       Print help information

COMMANDS:
    (none yet - real commands coming in future phases)

For more information, see: https://github.com/pRizz/opencode-cloud
`);
}

function printVersion(): void {
  const version = getVersionJs();
  console.log(version);
}

function main(): void {
  // Handle --version / -V
  if (args.includes("--version") || args.includes("-V")) {
    printVersion();
    process.exit(0);
  }

  // Handle --help / -h
  if (args.includes("--help") || args.includes("-h") || args.length === 0) {
    printHelp();
    process.exit(0);
  }

  // Unknown command
  console.error(`Unknown command: ${args.join(" ")}`);
  console.error('Run "opencode-cloud --help" for usage information.');
  process.exit(1);
}

main();
