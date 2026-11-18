#!/usr/bin/env node
// Check that the committed tree builds on its own, without uncommitted files from the working
// directory (refactor plan F1). Several people and tools commit from one checkout, so a commit can
// miss a file that only exists locally. CI catches this too (its clippy step compiles the
// checked-out commit); this script gives the same answer before pushing.
// Usage: node scripts/check-head-builds.mjs [revision]   (default HEAD)
import {execFileSync, spawnSync} from 'node:child_process'
import {mkdtempSync, rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join, posix, resolve} from 'node:path'
import {fileURLToPath} from 'node:url'

const root = resolve(fileURLToPath(import.meta.url), '../..')

/** Files outside crates/ that crate sources pull in (include!, include_str!, include_bytes!, #[path]) at `revision`. */
export function includedOutsideCrates(revision, cwd = root) {
  const listed = spawnSync('git', ['grep', '-lE', 'include(_str|_bytes)?!|#\\[path', revision, '--', 'crates/*.rs'],
    {cwd, encoding: 'utf8', maxBuffer: 1 << 28}).stdout.split('\n').filter(Boolean)
  const files = new Set()
  for (const entry of listed) {
    const file = entry.slice(revision.length + 1)
    const text = execFileSync('git', ['show', `${revision}:${file}`], {cwd, encoding: 'utf8', maxBuffer: 1 << 28})
    // The path literal may sit on the line after `include_str!(`.
    for (const m of text.matchAll(/(?:include(?:_str|_bytes)?!\(\s*|#\[path\s*=\s*)"([^"]+)"/g)) {
      const path = posix.normalize(posix.join(posix.dirname(file), m[1]))
      if (!path.startsWith('crates/') && !path.startsWith('..')) files.add(path)
    }
  }
  return [...files].sort()
}
function main() {
  const revision = process.argv[2] ?? 'HEAD'
  const exportDir = mkdtempSync(join(tmpdir(), 'head-build-'))
  // Reuse one target directory between runs so only changed crates rebuild.
  const target = process.env.CARGO_TARGET_DIR ?? resolve(root, 'crates/target/head-check')
  try {
    const archive = execFileSync('git', ['archive', revision, 'crates', 'rust-toolchain.toml', ...includedOutsideCrates(revision)],
      {cwd: root, maxBuffer: 1 << 30})
    execFileSync('tar', ['-x', '-C', exportDir], {input: archive, maxBuffer: 1 << 30})
    const check = spawnSync('cargo', ['check', '--locked', '--manifest-path', 'crates/Cargo.toml', '--workspace', '--all-targets'],
      {cwd: exportDir, stdio: 'inherit', env: {...process.env, CARGO_TARGET_DIR: target}})
    if (check.status !== 0) {
      console.error(`${revision} does not build on its own; a file it needs is probably uncommitted`)
      process.exitCode = check.status ?? 1
      return
    }
    console.log(`${revision} builds on its own`)
  } finally {
    rmSync(exportDir, {recursive: true, force: true})
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main()
