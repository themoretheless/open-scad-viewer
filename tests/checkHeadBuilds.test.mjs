import assert from 'node:assert/strict'
import {test} from 'node:test'
import {includedOutsideCrates} from '../scripts/check-head-builds.mjs'

test('the HEAD build check exports files that crates include from outside crates/', () => {
  const files = includedOutsideCrates('HEAD')
  // The path literal of this include sits on the line after `include_str!(`.
  assert.ok(files.includes('docs/languages/rush-nurbs-1.schema.json'))
  assert.ok(files.every(f => !f.startsWith('crates/') && !f.startsWith('..')))
})
