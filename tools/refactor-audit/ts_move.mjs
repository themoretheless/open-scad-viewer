#!/usr/bin/env node
// Move TypeScript/Svet files or directories and rewrite every relative import that pointed at
// them, plus the relative imports inside the moved files. Usage:
//   node tools/refactor-audit/ts_move.mjs moves.json [--dry]
// where moves.json is [["src/services/a.ts", "src/exchange/a.ts"], ["src/services/dir", "src/exchange/dir"]].
// Specifiers keep their style: an extensionless import stays extensionless, a directory import
// (resolved through index.ts) stays a directory import.
import {readFileSync, writeFileSync, existsSync, statSync, readdirSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {dirname, join, relative, resolve, normalize, posix} from 'node:path'

const root = resolve(dirname(new URL(import.meta.url).pathname), '../..')
const [movesPath, ...flags] = process.argv.slice(2)
const dry = flags.includes('--dry')
const moves = JSON.parse(readFileSync(movesPath, 'utf8')).map(([a, b]) => [normalize(a), normalize(b)])
const EXTS = ['.ts', '.tsx', '.mts', '.js', '.mjs', '.svt', '.vue', '.json']
const SCAN = ['src', 'tests', 'scripts', 'e2e'].filter(d => existsSync(join(root, d)))

const listed = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '--', ...SCAN], {cwd: root, encoding: 'utf8', maxBuffer: 1 << 28})
  .split('\n').filter(f => f && EXTS.some(e => f.endsWith(e)) && existsSync(join(root, f)))
const fileSet = new Set(listed)

// old path -> new path for every moved file
const newOf = new Map()
for (const [from, to] of moves) {
  const abs = join(root, from)
  if (!existsSync(abs)) throw new Error(`missing ${from}`)
  if (statSync(abs).isDirectory()) {
    for (const f of listed) if (f.startsWith(from + '/')) newOf.set(f, to + f.slice(from.length))
  } else newOf.set(from, to)
}
const moved = p => newOf.get(p) ?? p

function resolveSpec(fromFile, rawSpec) {
  const q = rawSpec.indexOf('?')
  const spec = q < 0 ? rawSpec : rawSpec.slice(0, q)
  const query = q < 0 ? '' : rawSpec.slice(q)
  const r = resolvePath(fromFile, spec)
  return r && {...r, query}
}
function resolvePath(fromFile, spec) {
  const base = posix.normalize(posix.join(posix.dirname(fromFile), spec))
  if (fileSet.has(base) || (existsSync(join(root, base)) && statSync(join(root, base)).isFile())) return {target: base, kind: 'exact'}
  for (const e of EXTS) if (fileSet.has(base + e)) return {target: base + e, kind: 'ext', ext: e}
  for (const e of EXTS) if (fileSet.has(base + '/index' + e)) return {target: base + '/index' + e, kind: 'dir'}
  return null
}
function respec(newFrom, newTarget, r, original) {
  let rel = posix.relative(posix.dirname(newFrom), newTarget)
  if (r.kind === 'ext') rel = rel.slice(0, -r.ext.length)
  if (r.kind === 'dir') rel = posix.dirname(rel)
  if (!rel.startsWith('.')) rel = './' + rel
  if (original.endsWith('/') && !rel.endsWith('/')) rel += '/'
  return rel + (r.query ?? '')
}
const SPEC = /(\bnew\s+URL\s*\(\s*|\bfrom\s*|\bimport\s*\(\s*|\bimport\s+|\bexport\s*\*\s*from\s*|\brequire\s*\(\s*|\bvi\.mock\s*\(\s*|\bvi\.importActual\s*(?:<[^>]*>)?\s*\(\s*)(['"])(\.{1,2}\/[^'"]*)\2/g
let rewrites = 0
const edits = new Map()
for (const f of listed) {
  const text = readFileSync(join(root, f), 'utf8')
  const newF = moved(f)
  let changed = false
  const out = text.replace(SPEC, (m, head, q, spec) => {
    const r = resolveSpec(f, spec)
    if (!r) return m
    const newTarget = moved(r.target)
    if (newF === f && newTarget === r.target) return m
    const next = respec(newF, newTarget, r, spec)
    if (next === spec) return m
    changed = true; rewrites++
    return head + q + next + q
  })
  if (changed) edits.set(f, out)
}
console.log(`${newOf.size} files move, ${rewrites} specifiers rewritten in ${edits.size} files`)
if (dry) process.exit(0)
for (const [f, out] of edits) writeFileSync(join(root, f), out)
for (const [from, to] of moves) {
  execFileSync('mkdir', ['-p', dirname(join(root, to))])
  // Other tools may hold the index lock briefly; retry instead of leaving a half-applied move.
  for (let attempt = 0; ; attempt++) {
    try { execFileSync('git', ['mv', from, to], {cwd: root, stdio: 'pipe'}); break }
    catch (error) {
      if (attempt > 60 || !String(error.stderr).includes('index.lock')) throw error
      execFileSync('sleep', ['2'])
    }
  }
}
