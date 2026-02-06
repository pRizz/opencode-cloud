import * as fs from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { describe, expect, it } from 'vitest';
import { ensureExecutable } from './permissions.js';

function makeTempBinary(mode: number): { dir: string; path: string } {
  const dir = fs.mkdtempSync(join(tmpdir(), 'occ-permissions-'));
  const path = join(dir, 'occ');
  fs.writeFileSync(path, '#!/usr/bin/env sh\necho ok\n', 'utf8');
  fs.chmodSync(path, mode);
  return { dir, path };
}

describe('ensureExecutable', () => {
  it('sets execute permissions when the file is not executable', () => {
    const tmp = makeTempBinary(0o644);

    ensureExecutable(tmp.path);

    const mode = fs.statSync(tmp.path).mode & 0o777;
    expect(mode & 0o111).not.toBe(0);

    fs.rmSync(tmp.dir, { recursive: true, force: true });
  });

  it('keeps executable files executable', () => {
    const tmp = makeTempBinary(0o755);

    ensureExecutable(tmp.path);

    const mode = fs.statSync(tmp.path).mode & 0o777;
    expect(mode & 0o111).not.toBe(0);

    fs.rmSync(tmp.dir, { recursive: true, force: true });
  });

  it('does not throw when chmod fails', () => {
    const tmp = makeTempBinary(0o644);
    const throwingFs = {
      statSync: (filePath: string) => fs.statSync(filePath),
      chmodSync: () => {
        throw new Error('chmod failed');
      },
    };

    expect(() => ensureExecutable(tmp.path, throwingFs)).not.toThrow();
    const mode = fs.statSync(tmp.path).mode & 0o777;
    expect(mode & 0o111).toBe(0);

    fs.rmSync(tmp.dir, { recursive: true, force: true });
  });
});
