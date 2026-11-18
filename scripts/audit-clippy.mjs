// Clippy ratchet: unique warnings per crate may only go down.
// Pins live in `architecture-baseline.json` under `clippy`; `--update` lowers them.
// `unwrap`/`expect` in production code (refactor plan L6) have their own per-crate pins under
// `clippyUnwrap`, so enabling those lints in a crate does not blur its general warning pin.
import {execFileSync} from 'node:child_process'
import {readFileSync, writeFileSync} from 'node:fs'
import {posix, resolve} from 'node:path'
import {fileURLToPath} from 'node:url'

export const UNWRAP_LINTS = new Set(['clippy::unwrap_used', 'clippy::expect_used'])

/** {crate: unique warning count} from `cargo clippy --message-format=json` output, for codes `keep` accepts. */
export function countClippy(jsonLines, keep = () => true) {
  const seen = new Set(), counts = {}
  for (const line of jsonLines.split('\n')) {
    if (!line.startsWith('{')) continue
    const event = JSON.parse(line)
    const message = event.reason === 'compiler-message' ? event.message : null
    if (!message || message.level !== 'warning' || !message.spans?.length) continue
    if (!keep(message.code?.code ?? '')) continue
    const span = message.spans.find(s => s.is_primary) ?? message.spans[0]
    if (span.file_name.startsWith('/')) continue // dependency or toolchain source
    // A file shared through `#[path]` is reported under each including crate's relative path.
    const file = posix.normalize(span.file_name)
    const key = `${message.code?.code ?? message.message}|${file}:${span.line_start}:${span.column_start}`
    if (seen.has(key)) continue
    seen.add(key)
    const crate = file.split('/')[0]
    counts[crate] = (counts[crate] ?? 0) + 1
  }
  return Object.fromEntries(Object.entries(counts).sort())
}

export function auditClippy(observed, pinned = {}, kind = 'clippy warnings') {
  const errors = []
  for (const [crate, count] of Object.entries(observed))
    if (count > (pinned[crate] ?? 0)) errors.push(`${crate}: ${count} ${kind} (pinned at ${pinned[crate] ?? 0}); fix the new ones`)
  for (const [crate, count] of Object.entries(pinned))
    if ((observed[crate] ?? 0) < count) errors.push(`${crate}: ${observed[crate] ?? 0} ${kind}, below the pin ${count}; lower it (--update)`)
  return errors
}

/** Lowered pins: never above the old pin, and no pin created for a crate the old pins omit. */
export function lowerPins(observed, pinned) {
  return Object.fromEntries(Object.entries(observed).map(([crate, count]) => [crate, Math.min(count, pinned[crate] ?? (pinned ? 0 : count))]).sort())
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const root = resolve(fileURLToPath(import.meta.url), '../..')
  const baselinePath = resolve(root, 'architecture-baseline.json')
  const output = execFileSync('cargo', ['clippy', '--locked', '--manifest-path', 'crates/Cargo.toml', '--workspace', '--all-targets',
    '--keep-going', '--message-format=json', '--', '--cap-lints', 'warn'], {cwd: root, encoding: 'utf8', maxBuffer: 1 << 30, stdio: ['ignore', 'pipe', 'inherit']})
  const observed = countClippy(output, code => !UNWRAP_LINTS.has(code))
  const unwrap = countClippy(output, code => UNWRAP_LINTS.has(code))
  const baseline = JSON.parse(readFileSync(baselinePath, 'utf8'))
  if (process.argv.includes('--update')) {
    baseline.clippy = lowerPins(observed, baseline.clippy ?? observed)
    baseline.clippyUnwrap = lowerPins(unwrap, baseline.clippyUnwrap ?? unwrap)
    writeFileSync(baselinePath, JSON.stringify(baseline, null, 2) + '\n')
  } else {
    const errors = [...auditClippy(observed, baseline.clippy), ...auditClippy(unwrap, baseline.clippyUnwrap, 'unwrap/expect warnings')]
    for (const error of errors) console.error(error)
    if (errors.length) process.exit(1)
    const total = counts => Object.values(counts).reduce((a, b) => a + b, 0)
    console.log(`clippy ok: ${total(observed)} warnings and ${total(unwrap)} unwrap/expect within per-crate pins`)
  }
}
