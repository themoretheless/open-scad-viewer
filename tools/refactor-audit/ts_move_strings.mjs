#!/usr/bin/env node
// Companion to ts_move.mjs: rewrite plain string literals that name moved source files, such as
// `new URL('../src/services/x.ts', import.meta.url)` lists in tests or `join(root, 'src/services/x.ts')`.
// Usage: node tools/refactor-audit/ts_move_strings.mjs moves1.json [moves2.json ...] [--dry]
// Every moves file is a list of [old, new] pairs that has already been applied with ts_move.mjs.
import {readFileSync, writeFileSync, existsSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {FROZEN_ARCHIVES} from '../../scripts/qualificationFrozenArchives.mjs'
import {dirname, join, resolve, posix} from 'node:path'

const root = resolve(dirname(new URL(import.meta.url).pathname), '../..')
const args = process.argv.slice(2)
const dry = args.includes('--dry')
const pairs = args.filter(a => a !== '--dry').flatMap(p => JSON.parse(readFileSync(p, 'utf8')))
const SCAN = ['src', 'tests', 'scripts', 'e2e', 'tools', 'benchmarks'].filter(d => existsSync(join(root, d)))
const EXTS = ['.ts', '.tsx', '.mts', '.js', '.mjs', '.svt', '.vue']
const files = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '--', ...SCAN], {cwd: root, encoding: 'utf8', maxBuffer: 1 << 28})
  .split('\n').filter(f => f && EXTS.some(e => f.endsWith(e)) && existsSync(join(root, f)))

// Immutable qualification archives keep their bytes (audit-architecture checks their hashes).
const frozen = new Set(Object.keys(FROZEN_ARCHIVES))
// old -> new for a path (file or anything under a moved directory)
const forward = p => {
  for (const [a, b] of pairs) {
    if (p === a) return b
    if (p.startsWith(a + '/')) return b + p.slice(a.length)
  }
  return p
}
// new -> old for the location of the file that contains the literal
const backward = p => {
  for (const [a, b] of pairs) {
    if (p === b) return a
    if (p.startsWith(b + '/')) return a + p.slice(b.length)
  }
  return p
}
const RESOLVE_EXTS = ['.ts', '.tsx', '.mts', '.js', '.mjs', '.svt', '.vue', '/index.ts']
const findOld = base => [''].concat(RESOLVE_EXTS).map(e => base + e).find(c => forward(c) !== c && existsSync(join(root, forward(c))))
// Extensionless relative imports that no longer resolve (files outside the directories ts_move scanned).
const IMP = /(\bfrom\s*|\bimport\s*\(\s*|\bvi\.mock\s*\(\s*)(['"])(\.{1,2}\/[^'"?]*)(\?[^'"]*)?\2/g
// Paths relative to src/ inside assertions, e.g. 'language/openscadCompiler.ts'.
const SRCREL = /(['"`])((?:services|semantic|renderer)\/[^'"`\n$]*?\.(?:ts|svt))\1/g
const LIT = /(['"`])((?:\.{1,2}\/|src\/)[^'"`\n$]*?\.(?:ts|tsx|svt|mjs|js|json|wgsl))\1/g
let n = 0
for (const f of files) {
  if (frozen.has(f)) continue
  const text = readFileSync(join(root, f), 'utf8')
  const oldF = backward(f)
  const out = text.replace(LIT, (m, q, lit) => {
    if (lit.startsWith('src/') || lit.startsWith('./src/')) {
      const bare = lit.replace(/^\.\//, '')
      const moved = forward(bare)
      const next = lit.startsWith('./') ? './' + moved : moved
      if (next === lit) return m
      n++; return q + next + q
    }
    // Already correct from the file's current location (for example an import ts_move fixed).
    if (existsSync(join(root, posix.normalize(posix.join(posix.dirname(f), lit))))) return m
    const oldTarget = posix.normalize(posix.join(posix.dirname(oldF), lit))
    const newTarget = forward(oldTarget)
    if (!existsSync(join(root, newTarget))) return m
    let rel = posix.relative(posix.dirname(f), newTarget)
    if (!rel.startsWith('.')) rel = './' + rel
    if (rel === lit) return m
    n++; return q + rel + q
  })
  const out2 = out.replace(IMP, (m, head, q, spec, query = '') => {
    if ([''].concat(RESOLVE_EXTS).some(e => existsSync(join(root, posix.normalize(posix.join(posix.dirname(f), spec)) + e)))) return m
    const old = findOld(posix.normalize(posix.join(posix.dirname(oldF), spec)))
    if (!old) return m
    const ext = RESOLVE_EXTS.find(e => old.endsWith(e) && old !== posix.normalize(posix.join(posix.dirname(oldF), spec)))
    let target = forward(old)
    if (ext === '/index.ts') target = posix.dirname(target)
    else if (ext) target = target.slice(0, -ext.length)
    let rel = posix.relative(posix.dirname(f), target)
    if (!rel.startsWith('.')) rel = './' + rel
    n++; return head + q + rel + query + q
  }).replace(SRCREL, (m, q, lit) => {
    const next = forward('src/' + lit)
    if (next === 'src/' + lit) return m
    n++; return q + next.slice(4) + q
  })
  if (out2 !== text && !dry) writeFileSync(join(root, f), out2)
}
console.log(`${n} string paths rewritten`)
