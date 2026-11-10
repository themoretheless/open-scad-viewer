// Clippy ratchet: unique warnings per crate may only go down.
// Pins live in `architecture-baseline.json` under `clippy`; `--update` lowers them.
import {execFileSync} from 'node:child_process'
import {readFileSync, writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {fileURLToPath} from 'node:url'

/** {crate: unique warning count} from `cargo clippy --message-format=json` output. */
export function countClippy(jsonLines) {
  const seen = new Set(), counts = {}
  for (const line of jsonLines.split('\n')) {
    if (!line.startsWith('{')) continue
    const event = JSON.parse(line)
    const message = event.reason === 'compiler-message' ? event.message : null
    if (!message || message.level !== 'warning' || !message.spans?.length) continue
    const span = message.spans.find(s => s.is_primary) ?? message.spans[0]
    if (span.file_name.startsWith('/')) continue // dependency or toolchain source
    const key = `${message.code?.code ?? message.message}|${span.file_name}:${span.line_start}:${span.column_start}`
    if (seen.has(key)) continue
    seen.add(key)
    const crate = span.file_name.split('/')[0]
    counts[crate] = (counts[crate] ?? 0) + 1
  }
  return Object.fromEntries(Object.entries(counts).sort())
}

export function auditClippy(observed, pinned = {}) {
  const errors = []
  for (const [crate, count] of Object.entries(observed))
    if (count > (pinned[crate] ?? 0)) errors.push(`${crate}: ${count} clippy warnings (pinned at ${pinned[crate] ?? 0}); fix the new ones`)
  for (const [crate, count] of Object.entries(pinned))
    if ((observed[crate] ?? 0) < count) errors.push(`${crate}: ${observed[crate] ?? 0} clippy warnings, below the pin ${count}; lower it (--update)`)
  return errors
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const root = resolve(fileURLToPath(import.meta.url), '../..')
  const baselinePath = resolve(root, 'architecture-baseline.json')
  const output = execFileSync('cargo', ['clippy', '--locked', '--manifest-path', 'crates/Cargo.toml', '--workspace', '--all-targets',
    '--keep-going', '--message-format=json', '--', '--cap-lints', 'warn'], {cwd: root, encoding: 'utf8', maxBuffer: 1 << 30, stdio: ['ignore', 'pipe', 'inherit']})
  const observed = countClippy(output)
  const baseline = JSON.parse(readFileSync(baselinePath, 'utf8'))
  if (process.argv.includes('--update')) {
    const pinned = baseline.clippy ?? observed
    baseline.clippy = Object.fromEntries(Object.entries(observed).map(([crate, count]) => [crate, Math.min(count, pinned[crate] ?? (baseline.clippy ? 0 : count))]).sort())
    writeFileSync(baselinePath, JSON.stringify(baseline, null, 2) + '\n')
  } else {
    const errors = auditClippy(observed, baseline.clippy)
    for (const error of errors) console.error(error)
    if (errors.length) process.exit(1)
    console.log(`clippy ok: ${Object.values(observed).reduce((a, b) => a + b, 0)} warnings within per-crate pins`)
  }
}
