// TypeScript/JavaScript lint ratchet: ESLint problems and Prettier-unformatted files per directory may only go down.
// Pins live in `architecture-baseline.json` under `eslint` and `prettier`; `--update` lowers them.
// Only tracked files are checked, so local untracked files never change the result.
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { auditClippy, lowerPins } from './audit-clippy.mjs'

export const LINT_EXTENSIONS = /\.(ts|mts|cts|js|mjs|cjs)$/
// Directories whose subdirectories are pinned separately, so a fix in one area cannot hide a regression in another.
const SPLIT_ROOTS = new Set(['src', 'tools', 'benchmarks', 'examples'])

/** Pin group of a repository-relative path: `src/kernel`, `tests`, `scripts`, or `.` for root files. */
export function lintGroup(file) {
  const parts = file.split('/')
  if (parts.length === 1) return '.'
  return SPLIT_ROOTS.has(parts[0]) && parts.length > 2 ? `${parts[0]}/${parts[1]}` : parts[0]
}

/** {group: count} for `[file, count]` pairs, sorted by group, omitting zero counts. */
export function countByGroup(entries) {
  const counts = {}
  for (const [file, count] of entries) if (count > 0) counts[lintGroup(file)] = (counts[lintGroup(file)] ?? 0) + count
  return Object.fromEntries(Object.entries(counts).sort(([a], [b]) => a.localeCompare(b)))
}

async function observe(root) {
  const { ESLint } = await import('eslint')
  const prettier = await import('prettier')
  const tracked = execFileSync('git', ['ls-files', '-z'], { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 })
    .split('\0')
    .filter(file => LINT_EXTENSIONS.test(file))
  const eslint = new ESLint({ cwd: root })
  const lintable = []
  for (const file of tracked) if (!(await eslint.isPathIgnored(file))) lintable.push(file)
  const results = await eslint.lintFiles(lintable)
  const eslintCounts = countByGroup(
    results.map(r => [
      r.filePath
        .slice(root.length + 1)
        .split('\\')
        .join('/'),
      r.messages.length,
    ]),
  )
  const ignorePath = resolve(root, '.prettierignore')
  const unformatted = []
  for (const file of tracked) {
    const path = resolve(root, file)
    if ((await prettier.getFileInfo(path, { ignorePath })).ignored) continue
    const options = { ...(await prettier.resolveConfig(path)), filepath: path }
    if (!(await prettier.check(readFileSync(path, 'utf8'), options))) unformatted.push([file, 1])
  }
  return { eslint: eslintCounts, prettier: countByGroup(unformatted) }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const root = resolve(fileURLToPath(import.meta.url), '../..')
  const baselinePath = resolve(root, 'architecture-baseline.json')
  const observed = await observe(root)
  const baseline = JSON.parse(readFileSync(baselinePath, 'utf8'))
  if (process.argv.includes('--update')) {
    baseline.eslint = lowerPins(observed.eslint, baseline.eslint ?? observed.eslint)
    baseline.prettier = lowerPins(observed.prettier, baseline.prettier ?? observed.prettier)
    writeFileSync(baselinePath, JSON.stringify(baseline, null, 2) + '\n')
  } else {
    const errors = [
      ...auditClippy(observed.eslint, baseline.eslint, 'ESLint problems'),
      ...auditClippy(observed.prettier, baseline.prettier, 'unformatted files (npx prettier --write <file>)'),
    ]
    if (errors.length) {
      console.error(errors.join('\n'))
      process.exit(1)
    }
    const total = counts => Object.values(counts).reduce((a, b) => a + b, 0)
    console.log(
      `lint ratchet ok: ${total(observed.eslint)} ESLint problems, ${total(observed.prettier)} unformatted files`,
    )
  }
}
