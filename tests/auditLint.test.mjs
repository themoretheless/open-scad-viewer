import assert from 'node:assert/strict'
import test from 'node:test'
import { countByGroup, lintGroup } from '../scripts/audit-lint.mjs'
import { auditClippy, lowerPins } from '../scripts/audit-clippy.mjs'

test('lint groups split src, tools, benchmarks and examples by subdirectory', () => {
  assert.equal(lintGroup('src/kernel/cadQuantity.ts'), 'src/kernel')
  assert.equal(lintGroup('src/main.ts'), 'src')
  assert.equal(lintGroup('tools/browser-qualification/run.mjs'), 'tools/browser-qualification')
  assert.equal(lintGroup('tests/support/helpers.ts'), 'tests')
  assert.equal(lintGroup('scripts/audit-lint.mjs'), 'scripts')
  assert.equal(lintGroup('vite.config.ts'), '.')
})

test('counts are summed per group, sorted, and zero counts are omitted', () => {
  assert.deepEqual(
    countByGroup([
      ['src/render/a.ts', 2],
      ['src/kernel/b.ts', 1],
      ['src/kernel/c.ts', 3],
      ['tests/d.test.ts', 0],
    ]),
    { 'src/kernel': 4, 'src/render': 2 },
  )
})

test('a new problem in any group fails the ratchet, a fix requires lowering the pin', () => {
  const pinned = { 'src/kernel': 4, scripts: 2 }
  assert.deepEqual(auditClippy({ 'src/kernel': 4, scripts: 2 }, pinned, 'ESLint problems'), [])
  assert.match(
    auditClippy({ 'src/kernel': 5, scripts: 2 }, pinned, 'ESLint problems')[0],
    /src\/kernel: 5 ESLint problems \(pinned at 4\)/,
  )
  assert.match(
    auditClippy({ 'src/kernel': 4, scripts: 2, tests: 1 }, pinned, 'ESLint problems')[0],
    /tests: 1 ESLint problems \(pinned at 0\)/,
  )
  assert.match(auditClippy({ 'src/kernel': 3, scripts: 2 }, pinned, 'ESLint problems')[0], /below the pin 4/)
  assert.deepEqual(lowerPins({ 'src/kernel': 3, scripts: 2, tests: 1 }, pinned), {
    'src/kernel': 3,
    scripts: 2,
    tests: 0,
  })
})
