/**
 * CLI Parity Tests
 *
 * Verifies the Node CLI wrapper can successfully invoke all commands from the Rust CLI.
 * Commands are discovered dynamically from `occ --help` to avoid hardcoded lists.
 */

import { execSync } from 'child_process';
import { describe, it, expect, beforeAll } from 'vitest';
import { join, resolve } from 'path';
import { existsSync } from 'fs';

// Path to the Rust binary (built from workspace)
const RUST_BINARY_PATH = resolve(__dirname, '../../../target/debug/opencode-cloud');

// Path where Node CLI expects to find the binary
const NODE_BIN_DIR = resolve(__dirname, '../bin');
const NODE_BIN_PATH = join(NODE_BIN_DIR, 'occ');

// Path to the Node CLI wrapper (built TypeScript)
const NODE_CLI_PATH = resolve(__dirname, '../dist/index.js');

/**
 * Parse commands from `occ --help` output
 */
function discoverCommands(): string[] {
  const helpOutput = execSync(`${RUST_BINARY_PATH} --help`, {
    encoding: 'utf-8',
  });

  const lines = helpOutput.split('\n');
  const commandsSection = lines.findIndex((line) => line.trim() === 'Commands:');

  if (commandsSection === -1) {
    throw new Error('Failed to find Commands: section in help output');
  }

  const commands: string[] = [];

  // Parse lines after "Commands:" until we hit "Options:"
  for (let i = commandsSection + 1; i < lines.length; i++) {
    const line = lines[i];

    // Stop at Options: section
    if (line.trim() === 'Options:') {
      break;
    }

    // Extract command name (first word after leading whitespace)
    const match = line.match(/^\s+([a-z-]+)\s+/);
    if (match) {
      commands.push(match[1]);
    }
  }

  return commands;
}

describe('CLI Parity', () => {
  let commands: string[];

  beforeAll(() => {
    // Ensure Rust binary exists
    if (!existsSync(RUST_BINARY_PATH)) {
      throw new Error(
        `Rust binary not found at ${RUST_BINARY_PATH}. Run: cargo build -p opencode-cloud`
      );
    }

    // Ensure Node CLI wrapper is built
    if (!existsSync(NODE_CLI_PATH)) {
      throw new Error(
        `Node CLI not built at ${NODE_CLI_PATH}. Run: pnpm -C packages/cli-node build`
      );
    }

    // Copy Rust binary to Node bin directory for passthrough
    execSync(`mkdir -p ${NODE_BIN_DIR}`);
    execSync(`cp ${RUST_BINARY_PATH} ${NODE_BIN_PATH}`);

    // Discover commands dynamically
    commands = discoverCommands();
  });

  it('should discover at least 10 commands', () => {
    expect(commands.length).toBeGreaterThanOrEqual(10);
  });

  it.each([
    'start',
    'stop',
    'restart',
    'status',
    'logs',
    'install',
    'uninstall',
    'config',
    'setup',
    'user',
    'mount',
    'update',
    'host',
  ])('should discover %s command', (command) => {
    expect(commands).toContain(command);
  });

  describe('Command Passthrough', () => {
    it('should pass --version through Node CLI', () => {
      const output = execSync(`node ${NODE_CLI_PATH} --version`, {
        encoding: 'utf-8',
      }).trim();

      expect(output).toMatch(/^opencode-cloud \d+\.\d+\.\d+$/);
    });

    it('should pass --help through Node CLI', () => {
      const output = execSync(`node ${NODE_CLI_PATH} --help`, {
        encoding: 'utf-8',
      });

      expect(output).toContain('Usage:');
      expect(output).toContain('Commands:');
      expect(output).toContain('Options:');
    });

    it.each([
      'start --help',
      'stop --help',
      'restart --help',
      'status --help',
      'logs --help',
      'install --help',
      'uninstall --help',
      'config --help',
      'setup --help',
      'user --help',
      'mount --help',
      'update --help',
      'host --help',
    ])('should pass "%s" through Node CLI', (commandWithArgs) => {
      const output = execSync(`node ${NODE_CLI_PATH} ${commandWithArgs}`, {
        encoding: 'utf-8',
      });

      // All help outputs should contain "Usage:" and describe the command
      expect(output).toContain('Usage:');
    });
  });
});
