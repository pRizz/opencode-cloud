#!/usr/bin/env node
/**
 * opencode-cloud Node.js CLI
 *
 * DEPRECATED: This package is deprecated. Please install via cargo instead.
 */

const RED = "\x1b[31m";
const YELLOW = "\x1b[33m";
const CYAN = "\x1b[36m";
const RESET = "\x1b[0m";
const BOLD = "\x1b[1m";

console.error(`
${YELLOW}${BOLD}Notice:${RESET} The npm package for opencode-cloud is deprecated.

Please install via cargo instead:

  ${CYAN}cargo install opencode-cloud${RESET}

This provides a native binary with better performance and full feature support.

${RED}Requires:${RESET} Rust 1.85+ (install from https://rustup.rs)
`);

process.exit(1);
