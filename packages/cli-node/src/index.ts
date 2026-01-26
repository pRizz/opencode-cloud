#!/usr/bin/env node
/**
 * opencode-cloud Node.js CLI
 *
 * Resolves the platform-specific binary from optionalDependencies,
 * with fallback to local bin/ for development.
 */

import { spawn, execSync } from 'child_process';
import { existsSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const scriptDir = dirname(fileURLToPath(import.meta.url));

/**
 * Detect if running on musl libc (Alpine Linux)
 */
function isMusl(): boolean {
  if (existsSync('/etc/alpine-release')) {
    return true;
  }
  try {
    const out = execSync('ldd --version 2>&1', { encoding: 'utf-8' });
    return out.includes('musl');
  } catch {
    return false;
  }
}

/**
 * Get the platform package name for the current environment
 */
function getPlatformPackage(): string | null {
  const { platform, arch } = process;

  if (platform === 'darwin') {
    return arch === 'arm64'
      ? '@opencode-cloud/cli-darwin-arm64'
      : '@opencode-cloud/cli-darwin-x64';
  }

  if (platform === 'linux') {
    const musl = isMusl();
    if (arch === 'x64') {
      return musl
        ? '@opencode-cloud/cli-linux-x64-musl'
        : '@opencode-cloud/cli-linux-x64';
    }
    if (arch === 'arm64') {
      return musl
        ? '@opencode-cloud/cli-linux-arm64-musl'
        : '@opencode-cloud/cli-linux-arm64';
    }
  }

  return null;
}

/**
 * Resolve path to the occ binary
 */
function resolveBinaryPath(): string | null {
  const platformPkg = getPlatformPackage();
  if (platformPkg) {
    try {
      const pkg = require(platformPkg);
      if (pkg.binaryPath && existsSync(pkg.binaryPath)) {
        return pkg.binaryPath;
      }
    } catch {
      // Package not installed, try fallback
    }
  }

  const localBinary = join(scriptDir, '..', 'bin', 'occ');
  if (existsSync(localBinary)) {
    return localBinary;
  }

  return null;
}

const binaryPath = resolveBinaryPath();

if (!binaryPath) {
  const platformPkg = getPlatformPackage();
  console.error('Error: Could not find opencode-cloud binary\n');
  console.error(`Platform: ${process.platform} (${process.arch})`);
  if (platformPkg) {
    console.error(`Expected package: ${platformPkg}\n`);
    console.error('This may indicate a failed npm install. Try:');
    console.error('  npm install opencode-cloud\n');
  } else {
    console.error('\nYour platform is not supported with prebuilt binaries.');
    console.error('Install the Rust CLI directly:');
    console.error('  cargo install opencode-cloud\n');
  }
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
});

child.on('close', (code: number | null) => {
  process.exit(code ?? 1);
});

child.on('error', (err: Error) => {
  console.error(`Error: Failed to spawn binary at ${binaryPath}`);
  console.error(`Error: ${err.message}\n`);
  console.error('Try reinstalling: npm install opencode-cloud');
  process.exit(1);
});
