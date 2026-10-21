import { mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

/**
 * Whether this process may create file symlinks. Windows requires Developer
 * Mode or the SeCreateSymbolicLinkPrivilege (granted on hosted CI runners);
 * without it symlink() fails with EPERM, so symlink rejection tests skip there.
 */
export const fileSymlinksAvailable: boolean = (() => {
  const root = mkdtempSync(join(tmpdir(), 'symlink-probe-'))
  try {
    writeFileSync(join(root, 'target'), '')
    symlinkSync(join(root, 'target'), join(root, 'link'))
    return true
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'EPERM') return false
    throw error
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})()
