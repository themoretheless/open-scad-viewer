import {test} from 'node:test'
import assert from 'node:assert/strict'
import {
  compiledTestTargets,
  cratesWithRunnableDoctests,
  hasRunnableDoctest,
  verifyNativePartition,
} from '../scripts/verify-native-test-partition.mjs'

const catalog = (target, names) => `     Running tests/cases.rs (${target})\n` + names.map(n => `${n}: test\n`).join('')
const full = catalog('/a', ['aag::recognition', 'shared']) + catalog('/b', ['shared']) + '   Doc-tests c\nexample: test\n'
const serial = catalog('/a', ['aag::recognition'])
const parallel = catalog('/a', ['shared']) + catalog('/b', ['shared']) + '   Doc-tests c\nexample: test\n'
test('counts equal names in separate targets and doc tests independently', () => {
  assert.deepEqual([verifyNativePartition(full, serial, parallel).full.cases,
    verifyNativePartition(full, serial, parallel).serial.cases,
    verifyNativePartition(full, serial, parallel).parallel.cases], [4, 1, 3])
  assert.equal(verifyNativePartition(full.replaceAll('\n', '\r\n'), serial, parallel).full.cases, 4)
})
test('refuses an omitted test', () => assert.throws(() => verifyNativePartition(full, serial,
  parallel.replace('example: test\n', '')), /Omitted test/u))
test('refuses overlapping or injected cases', () => {
  assert.throws(() => verifyNativePartition(full, serial, parallel + serial), /both partitions/u)
  assert.throws(() => verifyNativePartition(full, serial, parallel + catalog('/x', ['unexpected'])), /Unexpected parallel/u)
})
test('refuses duplicate, contextless and empty catalogs', () => {
  assert.throws(() => verifyNativePartition(full + full, serial, parallel), /Duplicate/u)
  assert.throws(() => verifyNativePartition('orphan: test\n', serial, parallel), /without a target/u)
  assert.throws(() => verifyNativePartition('', '', ''), /Empty complete/u)
})
test('extracts deduplicated test executables from cargo json output', () => {
  const json = [
    JSON.stringify({reason: 'compiler-artifact', profile: {test: false}, executable: '/tmp/lib'}),
    JSON.stringify({reason: 'compiler-artifact', profile: {test: true}, executable: '/tmp/t1', target: {src_path: '/repo/crates/a/src/lib.rs'}}),
    JSON.stringify({reason: 'compiler-artifact', profile: {test: true}, executable: '/tmp/t1', target: {src_path: '/repo/crates/a/src/lib.rs'}}),
    JSON.stringify({reason: 'compiler-artifact', profile: {test: true}, executable: '/tmp/t2', target: {src_path: '/repo/crates/b/tests/it.rs'}}),
  ].join('\n')
  assert.deepEqual(compiledTestTargets(json, '/repo'), [
    {srcPath: 'crates/a/src/lib.rs', executable: '/tmp/t1'},
    {srcPath: 'crates/b/tests/it.rs', executable: '/tmp/t2'},
  ])
})
test('identifies runnable rust doctest fences vs non-rust text fences', () => {
  assert.equal(hasRunnableDoctest('/// ```text\n/// plain diagram\n/// ```\n'), false)
  assert.equal(hasRunnableDoctest('//! ```sh\n//! cargo run\n//! ```\n'), false)
  assert.equal(hasRunnableDoctest('/// ```\n/// assert_eq!(1 + 1, 2);\n/// ```\n'), true)
  assert.equal(hasRunnableDoctest('//! ```no_run\n//! let _ = 1;\n//! ```\n'), true)
  const doctestCrates = cratesWithRunnableDoctests()
  assert(doctestCrates.includes('geometry-ops'))
  assert(doctestCrates.includes('planar-geometry'))
  assert(!doctestCrates.includes('nurbs-core'))
})
