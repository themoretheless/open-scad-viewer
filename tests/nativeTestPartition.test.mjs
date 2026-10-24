import {test} from 'node:test'
import assert from 'node:assert/strict'
import {verifyNativePartition} from '../scripts/verify-native-test-partition.mjs'

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
