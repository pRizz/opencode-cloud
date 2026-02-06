import * as fs from 'fs';

type PermissionFs = {
  statSync: (filePath: string) => { mode: number };
  chmodSync: (filePath: string, mode: number) => void;
};

/**
 * Best-effort permission fix for packaged binaries.
 * If execute bits are missing, attempt to restore them.
 */
export function ensureExecutable(
  filePath: string,
  fileSystem: PermissionFs = fs
): void {
  try {
    const mode = fileSystem.statSync(filePath).mode;
    if ((mode & 0o111) === 0) {
      fileSystem.chmodSync(filePath, 0o755);
    }
  } catch {
    // Do not hard-fail here; spawn() will report a concrete error.
  }
}
