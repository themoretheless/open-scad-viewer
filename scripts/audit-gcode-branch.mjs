import assert from 'node:assert/strict'
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {dirname, join} from 'node:path'
import {fileURLToPath} from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const commit = 'a96efc1793f17b77c5494c9a5d8b7fcd1c05ff8b'
const scratch = mkdtempSync(join(tmpdir(), 'osv-gcode-state-audit-'))
const run = (command, args) => execFileSync(command, args, {cwd: root, maxBuffer: 8 * 1024 * 1024, timeout: 120_000})
const git = (...args) => run('git', ['-c', 'core.fsmonitor=false', ...args])
try {
  const paths = git('ls-tree', '-r', '--name-only', commit, '--', 'crates/gcode-core/src', 'crates/math-core/src')
    .toString().trim().split('\n')
  const sources = paths.map(path => {
    assert.match(path, /^crates\/(gcode-core|math-core)\/src\/[a-z_]+\.rs$/)
    const bytes = git('show', `${commit}:${path}`)
    const output = join(scratch, path)
    mkdirSync(dirname(output), {recursive: true})
    writeFileSync(output, bytes)
    return {path, sha256: createHash('sha256').update(bytes).digest('hex')}
  })
  // Compile the original modules directly: no workspace/dependency overlay,
  // source rewriting, network resolution or production crate mutation.
  const math = join(scratch, 'libmath_core.rlib'), core = join(scratch, 'libgcode_core.rlib')
  run('rustc', ['--edition=2024', '--crate-type=rlib', '--crate-name=math_core', join(scratch, 'crates/math-core/src/lib.rs'), '-o', math])
  run('rustc', ['--edition=2024', '--crate-type=rlib', '--crate-name=gcode_core', join(scratch, 'crates/gcode-core/src/lib.rs'), '--extern', `math_core=${math}`, '-o', core])
  const fixture = join(root, 'scripts/fixtures/gcode-legacy-state.rs'), binary = join(scratch, 'probe')
  run('rustc', ['--edition=2024', fixture, '--extern', `gcode_core=${core}`, '-L', `dependency=${scratch}`, '-o', binary])
  const observed = JSON.parse(run(binary, []).toString())
  console.log(JSON.stringify({commit, rustc: run('rustc', ['--version']).toString().trim(),
    scope: 'Pinned legacy interpreter defect reproduction, not production validation or firmware execution',
    qualificationClaim: 'none', sources,
    fixtureSha256: createHash('sha256').update(readFileSync(fixture)).digest('hex'), observed}, null, 2))
} finally { rmSync(scratch, {recursive: true, force: true}) }
