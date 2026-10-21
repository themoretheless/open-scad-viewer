import assert from 'node:assert/strict'

export const legacyCapRefusals = new Set([
  'open-spatial-hollow-miter.step',
  'affine-oblique-hollow-miter.step',
])

// Reconcile captured Rust evidence only. An external tolerance-based STEP
// import does not turn a proved exact coedge mismatch into an admitted Solid.
export function checkLegacyCapRefusal(item) {
  assert.ok(legacyCapRefusals.has(item.file), 'Unknown legacy cap refusal')
  assert.ok(Number.isSafeInteger(item.faces) && item.faces > 0)
  assert.equal(item.faceLoops?.length, item.faces, 'Incomplete face-loop ownership')
  assert.ok(Array.isArray(item.capFaces) && item.capFaces.length > 0)
  const caps = new Set(item.capFaces)
  assert.equal(caps.size, item.capFaces.length, 'Duplicate cap face')
  for (const face of caps) assert.ok(Number.isInteger(face) && face >= 0 && face < item.faces)
  const uses = item.nativeExactUses
  assert.ok(Array.isArray(uses) && uses.length > 0, 'Missing exact coedge evidence')
  const owned = Array.from({length: item.faces}, () => new Map())
  const seen = new Set()
  for (const use of uses) {
    assert.ok(Number.isInteger(use.face) && use.face >= 0 && use.face < item.faces)
    assert.ok(Number.isInteger(use.wire) && use.wire >= 0)
    assert.ok(Number.isInteger(use.coedge) && use.coedge >= 0)
    const key = `${use.face}:${use.wire}:${use.coedge}`
    assert.ok(!seen.has(key), 'Duplicate exact coedge evidence')
    seen.add(key)
    const wires = owned[use.face]
    if (!wires.has(use.wire)) wires.set(use.wire, [])
    wires.get(use.wire).push(use)
  }
  for (let face = 0; face < item.faces; face++) {
    const loops = item.faceLoops[face]
    const expected = [loops.outer, ...loops.holes].map(edges => [...edges])
    assert.equal(owned[face].size, expected.length, 'Incomplete wire evidence')
    for (const wire of owned[face].values()) {
      wire.sort((a, b) => a.coedge - b.coedge)
      assert.deepEqual(wire.map(use => use.coedge), wire.map((_, i) => i), 'Incomplete coedge sequence')
      const edges = wire.map(use => use.edge)
      const index = expected.findIndex(loop => loop.length === edges.length && loop.every((edge, i) => edge === edges[i]))
      assert.ok(index >= 0, 'Coedge evidence does not belong to captured face loop')
      expected.splice(index, 1)
    }
    assert.equal(expected.length, 0)
  }
  const gaps = uses.filter(use => use.status !== 'equal')
  assert.ok(gaps.length > 0, 'Missing proved cap mismatch')
  for (const use of gaps) {
    assert.equal(use.status, 'different', 'Unresolved identity is not a proved cap mismatch')
    assert.equal(use.exactIdentityCertified, false)
    assert.equal(use.globalEmbeddingCertified, false)
    assert.ok(caps.has(use.face), 'Mismatch is outside cap scope')
    assert.ok(Number.isSafeInteger(use.work) && use.work > 0 && use.work <= 1000000)
    assert.equal(use.independent, null)
  }
  for (const use of uses.filter(use => use.status === 'equal')) assert.equal(use.exactIdentityCertified, true)
  assert.equal(item.nativeBoundary?.exactBoundaryCertified, false)
  assert.equal(item.nativeVolume?.solidGeometryCertified, false)
  assert.equal(item.nativeVolume.boundaryEmbeddingCertified, false)
  assert.equal(item.nativeVolume.nesting, null)
  assert.equal(item.nativeVolume.orientations?.length, item.shells)
  assert.ok(item.nativeVolume.orientations.every(orientation => orientation.outward === null))
  return {file: item.file, provedCapMismatches: gaps.length, auditedCoedges: uses.length}
}
