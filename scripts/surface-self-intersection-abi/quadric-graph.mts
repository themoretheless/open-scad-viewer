import assert from 'node:assert/strict'
import {
  quadricGraphCases,
  graphQuery,
  boundaryQuery,
  foldedJointQuery,
  multispanJointRefusalQuery,
} from '../../tools/browser-qualification/quadric-graph-fixtures.ts'
import {
  isOriginalQuadricGraphReport,
  type OriginalQuadricGraphReport,
} from '../../src/kernel/originalQuadricGraphProtocol.ts'
import { cases, request, reject } from './abi.mts'

let graphBaseline: OriginalQuadricGraphReport | undefined
for (const fixture of quadricGraphCases) {
  const before = JSON.stringify(fixture.query),
    started = performance.now(),
    reply = request({ op: 'nurbs_original_quadric_graph_contacts', ...fixture.query }),
    elapsedMs = performance.now() - started
  assert.equal(reply.ok, true)
  assert(isOriginalQuadricGraphReport(reply.value))
  fixture.check(reply.value)
  assert.equal(JSON.stringify(fixture.query), before)
  if (fixture.name === 'graph-edge-0') graphBaseline = reply.value
  cases.push({
    name: fixture.name,
    elapsedMs,
    status: reply.value.status,
    complete: reply.value.curvePatchCoverageComplete,
    contacts: reply.value.contacts,
    exactWork: reply.value.exactWork,
  })
}
assert(graphBaseline?.carrier)
const graphBudgetQuery = {
  ...boundaryQuery(),
  maxExactWork: graphBaseline.targetMembership.workUsed + graphBaseline.carrier.exactWork,
}
const graphBudgetReply = request({ op: 'nurbs_original_quadric_graph_contacts', ...graphBudgetQuery })
assert.equal(graphBudgetReply.ok, true)
assert(isOriginalQuadricGraphReport(graphBudgetReply.value))
assert.equal(graphBudgetReply.value.status, 'unresolved')
assert.equal(graphBudgetReply.value.finiteSurfaceUvAdmission, false)
assert.equal(graphBudgetReply.value.carrier?.curveCarrierCoverageComplete, true)
cases.push({
  name: 'graph-boundary-budget-exhaustion',
  status: graphBudgetReply.value.status,
  exactWork: graphBudgetReply.value.exactWork,
})
reject('graph-forged-body', { op: 'nurbs_original_quadric_graph_contacts', ...graphQuery(), bodyAdmission: true })
reject('graph-forged-target-proof', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...graphQuery(),
  targetGraphProven: true,
})
reject('graph-over-limit-work', { op: 'nurbs_original_quadric_graph_contacts', ...graphQuery(), maxExactWork: 1000001 })
function refuseOriginalGraph(name: string, v: object): void {
  const started = performance.now(),
    reply = request(v),
    elapsedMs = performance.now() - started
  assert.equal(reply.ok, true)
  assert(isOriginalQuadricGraphReport(reply.value))
  const r = reply.value
  assert.equal(r.targetParameterization, 'joint-jacobian')
  assert.equal(r.targetMembership.status, 'different')
  assert.equal(r.targetGraphProven, false)
  assert.equal(r.finiteSurfaceUvAdmission, false)
  assert.equal(r.carrier, null)
  assert.deepEqual(r.contacts, [])
  cases.push({
    name,
    status: r.status,
    targetParameterization: r.targetParameterization,
    targetMembership: r.targetMembership,
    elapsedMs,
  })
}
const graphBadWeights = graphQuery()
graphBadWeights.patch.weights[1]![1] = 2
refuseOriginalGraph('graph-unsupported-target-weights', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...graphBadWeights,
})
const graphBadSheet = graphQuery()
graphBadSheet.quadric = [
  [0, 0, 0, 0],
  [0, 0, 0, 0],
  [0, 0, 1, 0],
  [0, 0, 0, -1],
]
reject('graph-ambiguous-sheet', { op: 'nurbs_original_quadric_graph_contacts', ...graphBadSheet })
const graphPerturbedRank = graphQuery()
graphPerturbedRank.patch.weights = [
  [1, 3],
  [2, 6 + Number.EPSILON * 4],
]
refuseOriginalGraph('graph-perturbed-separable-target', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...graphPerturbedRank,
})
const multiBase = quadricGraphCases.find(c => c.name === 'graph-multispan-internal-knot')!
const multiDrift = structuredClone(multiBase.query)
multiDrift.patch.controlPoints[1]![0]![0] = 0.25 + Number.EPSILON / 4
refuseOriginalGraph('graph-multispan-ulp-projection-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...multiDrift,
})
const multiRational = structuredClone(multiBase.query)
multiRational.patch.weights[1] = [14, 14, 14, 14]
const multiRationalReply = request({ op: 'nurbs_original_quadric_graph_contacts', ...multiRational })
assert.equal(multiRationalReply.ok, true)
assert(isOriginalQuadricGraphReport(multiRationalReply.value))
assert.equal(multiRationalReply.value.status, 'present-on-patch')
assert.equal(multiRationalReply.value.targetParameterization, 'separable-rational')
cases.push({
  name: 'graph-multispan-rational-clock',
  status: multiRationalReply.value.status,
  contacts: multiRationalReply.value.contacts,
})
const homogeneousDrift = structuredClone(
  quadricGraphCases.find(c => c.name === 'graph-homogeneous-nonplanar-rotated-false-gauge-1')!.query,
)
homogeneousDrift.patch.controlPoints[1]![1]![0] = 6 + Number.EPSILON * 4
refuseOriginalGraph('graph-homogeneous-original-ulp-projection-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...homogeneousDrift,
})
const multispanHomogeneousDrift = structuredClone(
  quadricGraphCases.find(c => c.name === 'graph-multispan-homogeneous-rotated-false-gauge-1')!.query,
)
multispanHomogeneousDrift.patch.controlPoints[1]![1]![0]! += 2 ** -34
refuseOriginalGraph('graph-multispan-homogeneous-original-ulp-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...multispanHomogeneousDrift,
})
const homogeneousSeparableDrift = structuredClone(
  quadricGraphCases.find(c => c.name === 'graph-homogeneous-separable-rotated-false-gauge-1')!.query,
)
homogeneousSeparableDrift.patch.controlPoints[1]![1]![0]! += 2 ** -47
refuseOriginalGraph('graph-homogeneous-separable-original-ulp-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...homogeneousSeparableDrift,
})
const multispanBoundaryClockDrift = structuredClone(
  quadricGraphCases.find(c => c.name === 'graph-multispan-boundary-clocks-rotated-false-gauge-1-denominator-3')!.query,
)
multispanBoundaryClockDrift.patch.controlPoints[1]![1]![0]! += 2 ** -29
refuseOriginalGraph('graph-multispan-boundary-clocks-original-ulp-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...multispanBoundaryClockDrift,
})
const foldedClock = structuredClone(
  quadricGraphCases.find(c => c.name === 'graph-unordered-monotone-kind-0-gauge-1-rotated-false')!.query,
)
for (const [i, x] of [0, 2, -1, 1].entries()) {
  foldedClock.patch.controlPoints[i]![0]![0] = x
  foldedClock.patch.controlPoints[i]![1]![0] = x
  foldedClock.patch.controlPoints[i]![1]![2] = x
}
reject('graph-folded-clock-positive-endpoint-slopes', { op: 'nurbs_original_quadric_graph_contacts', ...foldedClock })
const triangularDrift = structuredClone(
  quadricGraphCases.find(c => c.name === 'graph-triangular-transpose-false-rotated-false-gauge-1')!.query,
)
triangularDrift.patch.controlPoints[1]![1]![0] = 1 + Number.EPSILON
refuseOriginalGraph('graph-triangular-original-identity-ulp-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...triangularDrift,
})
const triangularMultispanDrift = structuredClone(
  quadricGraphCases.find(c => c.name === 'graph-triangular-multispan-point-0-transpose-false-rotated-false-gauge-1')!
    .query,
)
triangularMultispanDrift.patch.controlPoints[1]![1]![0]! += 2 ** -44
refuseOriginalGraph('graph-triangular-multispan-original-identity-ulp-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...triangularMultispanDrift,
})
reject('graph-joint-original-folded-map', { op: 'nurbs_original_quadric_graph_contacts', ...foldedJointQuery() })
const jointBoundaryDrift = structuredClone(
  quadricGraphCases.find(c => c.name === 'graph-joint-polynomial-transpose-false-rotated-false-gauge-1')!.query,
)
jointBoundaryDrift.patch.controlPoints[0]![1]![0] = Number.MIN_VALUE
reject('graph-joint-original-boundary-ulp-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...jointBoundaryDrift,
})
for (const kind of ['folded', 'discontinuous'] as const)
  reject('graph-joint-multispan-' + kind, {
    op: 'nurbs_original_quadric_graph_contacts',
    ...multispanJointRefusalQuery(kind),
  })
const jointMultispanBoundaryDrift = structuredClone(
  quadricGraphCases.find(
    c => c.name === 'graph-joint-multispan-polynomial-point-0-transpose-false-rotated-false-gauge-1',
  )!.query,
)
jointMultispanBoundaryDrift.patch.controlPoints[0]![1]![0] = Number.MIN_VALUE
reject('graph-joint-multispan-original-boundary-drift', {
  op: 'nurbs_original_quadric_graph_contacts',
  ...jointMultispanBoundaryDrift,
})
