import assert from 'node:assert/strict'
import {readFileSync, writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {checkLegacyCapRefusal, legacyCapRefusals} from './sweep-legacy-cap-refusal-contracts.mjs'

const [input, output] = process.argv.slice(2)
assert.ok(input && output, 'Usage: captured-cases.json report.json')
const bytes = readFileSync(input), captured = JSON.parse(bytes)
assert.deepEqual(new Set(captured.cases.map(item => item.file)), legacyCapRefusals)
const controls = [
  ['omitted coedge', item => item.nativeExactUses.pop()],
  ['duplicate coedge', item => item.nativeExactUses.push(structuredClone(item.nativeExactUses[0]))],
  ['foreign edge', item => item.nativeExactUses[0].edge = -1],
  ['unresolved mismatch', item => item.nativeExactUses.find(use => use.status === 'different').status = 'unresolved'],
  ['no proved mismatch', item => {for (const use of item.nativeExactUses) {use.status = 'equal'; use.exactIdentityCertified = true}}],
  ['unproved equal use', item => item.nativeExactUses.find(use => use.status === 'equal').exactIdentityCertified = false],
  ['zero-work mismatch', item => item.nativeExactUses.find(use => use.status === 'different').work = 0],
  ['mismatch outside caps', item => item.capFaces = [0]],
  ['promoted exact boundary', item => item.nativeBoundary.exactBoundaryCertified = true],
  ['promoted Solid', item => item.nativeVolume.solidGeometryCertified = true],
  ['promoted orientation', item => item.nativeVolume.orientations[0].outward = true],
]
const cases = []
for (const item of captured.cases) {
  const before = JSON.stringify(item), proof = checkLegacyCapRefusal(item)
  assert.equal(JSON.stringify(item), before, 'Reconciliation must not mutate captured evidence')
  const refused = []
  for (const [name, mutate] of controls) {
    const changed = structuredClone(item)
    mutate(changed)
    assert.throws(() => checkLegacyCapRefusal(changed), undefined, name)
    refused.push(name)
  }
  cases.push({...proof, refused})
}
const sha = value => createHash('sha256').update(value).digest('hex')
writeFileSync(output, JSON.stringify({schema: 'legacy-cap-refusal-controls/1', passed: true,
  scope: 'Reconciliation of captured native exact mismatch evidence and mutation controls; not fresh geometry qualification',
  artifactProvenance: captured.artifactProvenance, manifestSha256: captured.manifestSha256,
  capturedInputSha256: sha(bytes), checkerSha256: sha(readFileSync(new URL('./sweep-legacy-cap-refusal-contracts.mjs', import.meta.url))),
  runnerSha256: sha(readFileSync(new URL(import.meta.url))), cases, fullGoalComplete: false}, null, 2) + '\n')
console.log(`${cases.length} actual captured cap refusals and ${cases.length * controls.length} mutation controls passed`)
