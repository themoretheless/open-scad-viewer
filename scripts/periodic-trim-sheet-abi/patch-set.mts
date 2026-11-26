import assert from 'node:assert/strict'
import { call, request, surfaceAreaOptions } from './fixture.mts'

const patchSetOp = 'brep_measure_exact_periodic_trim_patch_set_mass_properties'
const island = [
  [0.1875, 0.375],
  [0.3125, 0.375],
  [0.3125, 0.625],
  [0.1875, 0.625],
  [0.1875, 0.375],
]
const patchSetOptions = {
  ...surfaceAreaOptions,
  contours: [...request.contours, island],
  maxTrimCells: 1_000_000,
}
const patchSet = call(request.maxWork, patchSetOp, { ...patchSetOptions, maxSurfaceCells: 20_000 })
assert.equal(patchSet.componentAreas?.length, 2)
assert.equal(patchSet.componentAreas?.[1].outerContour, 2)
const islandBounds = patchSet.componentAreas?.[1].boundsMm2
assert.ok(islandBounds && islandBounds[0] <= 0.0625 && 0.0625 <= islandBounds[1])
assert.ok(patchSet.centroidBoundsMm)
assert.equal(patchSet.centralInertiaBoundsMm4?.length, 3)
const incompleteSet = call(request.maxWork, patchSetOp, { ...patchSetOptions, maxSurfaceCells: 2 })
assert.equal(incompleteSet.status, 'workLimit')
assert.deepEqual(incompleteSet.componentAreas, [])
assert.equal(incompleteSet.boundsMm2, null)
console.log('WASM ABI returned complete multi-component trimmed surface mass properties.')

assert.equal(patchSet.principalMomentBoundsMm4, undefined)
const spectralOptions = { ...patchSetOptions, maxSurfaceCells: 20_000 }
for (const [momentToleranceMm4, met] of [
  [10, true],
  [1e-8, false],
]) {
  const spectrum = call(request.maxWork, patchSetOp, { ...spectralOptions, momentToleranceMm4 })
  assert.equal(spectrum.principalMomentBoundsMm4?.length, 3)
  assert.equal(spectrum.withinMomentTolerance, met)
}
const incompleteSpectrum = call(request.maxWork, patchSetOp, {
  ...spectralOptions,
  maxSurfaceCells: 2,
  momentToleranceMm4: 10,
})
assert.equal(incompleteSpectrum.principalMomentBoundsMm4, null)
assert.equal(incompleteSpectrum.withinMomentTolerance, false)
console.log('WASM ABI returned optional principal moments for the complete patch set.')

assert.equal(patchSet.axisMomentBoundsMm4, undefined)
const axisOptions = {
  ...spectralOptions,
  momentToleranceMm4: 10,
  inertiaAxis: { pointMm: [1, 0, 5], direction: [0, 0, -2] },
}
const axisMass = call(request.maxWork, patchSetOp, axisOptions)
const expectedAxisMoment = 83 / 768 + 49 / 12288 + 1.3125 * (1 - (2 * 3) / 28)
const axisBounds = axisMass.axisMomentBoundsMm4
assert.ok(axisBounds && axisBounds[0] <= expectedAxisMoment && expectedAxisMoment <= axisBounds[1])
assert.equal(axisMass.principalMomentBoundsMm4?.length, 3)
assert.equal(call(request.maxWork, patchSetOp, { ...axisOptions, maxSurfaceCells: 2 }).axisMomentBoundsMm4, null)
console.log('WASM ABI enclosed the patch set moment about an offset axis.')

assert.equal(patchSet.referenceInertiaBoundsMm4, undefined)
const referenceOptions = { ...axisOptions, inertiaReferencePointMm: [1, 0, 0] }
const referenceMass = call(request.maxWork, patchSetOp, referenceOptions)
const referenceTensor = referenceMass.referenceInertiaBoundsMm4
assert.ok(referenceTensor)
assert.deepEqual(referenceTensor[0][1], referenceTensor[1][0])
assert.ok(referenceTensor[2][2][0] <= expectedAxisMoment && expectedAxisMoment <= referenceTensor[2][2][1])
assert.deepEqual(referenceMass.axisMomentBoundsMm4, axisMass.axisMomentBoundsMm4)
const incompleteReference = call(request.maxWork, patchSetOp, { ...referenceOptions, maxSurfaceCells: 2 })
assert.equal(incompleteReference.referenceInertiaBoundsMm4, null)
console.log('WASM ABI enclosed the patch set tensor about a fixed reference point.')

assert.equal(axisMass.axisRadiusOfGyrationBoundsMm, undefined)
const gyrationOptions = { ...axisOptions, includeAxisRadiusOfGyration: true }
const gyrationMass = call(request.maxWork, patchSetOp, gyrationOptions)
const gyrationBounds = gyrationMass.axisRadiusOfGyrationBoundsMm
const expectedGyration = Math.sqrt(expectedAxisMoment / 1.3125)
assert.ok(gyrationBounds && gyrationBounds[0] <= expectedGyration && expectedGyration <= gyrationBounds[1])
assert.deepEqual(gyrationMass.axisMomentBoundsMm4, axisMass.axisMomentBoundsMm4)
const incompleteGyration = call(request.maxWork, patchSetOp, { ...gyrationOptions, maxSurfaceCells: 2 })
assert.equal(incompleteGyration.axisRadiusOfGyrationBoundsMm, null)
console.log('WASM ABI enclosed the patch set radius of gyration about an offset axis.')

const densityOp = 'brep_measure_exact_periodic_trim_patch_set_density'
const densityOptions = {
  ...spectralOptions,
  componentDensities: [
    { outerContour: 2, densityBoundsKgPerMm2: [6, 6] },
    { outerContour: 0, densityBoundsKgPerMm2: [2, 2] },
  ],
}
const densityMass = call(request.maxWork, densityOp, densityOptions)
const expectedMass = 2.875
const expectedCenterX = 0.34375 / expectedMass
const expectedCenterY = 0.375 / expectedMass
const expectedWeightedInertia =
  (2 * 83) / 768 + (6 * 49) / 12288 - expectedMass * (expectedCenterX ** 2 + expectedCenterY ** 2)
for (const [bounds, expected] of [
  [densityMass.massBoundsKg, expectedMass],
  [densityMass.centerOfMassBoundsMm?.[0], expectedCenterX],
  [densityMass.centralInertiaBoundsKgMm2?.[2]?.[2], expectedWeightedInertia],
] as const) {
  assert.ok(bounds && bounds[0] <= expected && expected <= bounds[1])
}
assert.equal(call(request.maxWork, densityOp, { ...densityOptions, maxSurfaceCells: 2 }).massBoundsKg, null)
console.log('WASM ABI enclosed density-weighted mass properties by outer contour.')

assert.deepEqual(
  densityMass.componentMasses?.map(({ outerContour }) => outerContour),
  [0, 2],
)
for (const [component, expected] of [
  [0, 2.5],
  [1, 0.375],
]) {
  const bounds = densityMass.componentMasses?.[component]?.boundsKg
  assert.ok(bounds && bounds[0] <= expected && expected <= bounds[1])
}
assert.deepEqual(call(request.maxWork, densityOp, { ...densityOptions, maxSurfaceCells: 2 }).componentMasses, [])
console.log('WASM ABI enclosed the mass of each retained component.')

for (const [component, expected] of [
  [0, 20 / 23],
  [1, 3 / 23],
]) {
  const fraction = densityMass.componentMasses?.[component]?.massFractionBounds
  assert.ok(fraction && 0 <= fraction[0] && fraction[0] <= expected && expected <= fraction[1] && fraction[1] <= 1)
}
console.log('WASM ABI enclosed the mass fraction of each retained component.')

assert.equal(densityMass.centerOfMassPlaneDistanceBoundsMm, undefined)
const planeOptions = {
  ...densityOptions,
  centerOfMassPlane: { pointMm: [1, 0, 0], normal: [2, -2, 0] },
}
const planeResult = call(request.maxWork, densityOp, planeOptions)
const centerPlane = planeResult.centerOfMassPlaneDistanceBoundsMm
const expectedPlaneDistance = (expectedCenterX - expectedCenterY - 1) / Math.SQRT2
assert.ok(centerPlane && centerPlane[0] <= expectedPlaneDistance && expectedPlaneDistance <= centerPlane[1])
assert.ok(centerPlane[1] < 0)
const incompletePlane = call(request.maxWork, densityOp, { ...planeOptions, maxSurfaceCells: 2 })
assert.equal(incompletePlane.centerOfMassPlaneDistanceBoundsMm, null)
const firstMassMoment = planeResult.firstMassMomentAboutPlaneBoundsKgMm
const expectedFirstMassMoment = expectedPlaneDistance * 2.875
assert.ok(
  firstMassMoment && firstMassMoment[0] <= expectedFirstMassMoment && expectedFirstMassMoment <= firstMassMoment[1],
)
assert.equal(incompletePlane.firstMassMomentAboutPlaneBoundsKgMm, null)
assert.equal(densityMass.firstMassMomentAboutPlaneBoundsKgMm, undefined)
console.log('WASM ABI enclosed the signed first mass moment about a plane.')
console.log('WASM ABI enclosed the signed distance from the density-weighted center to a plane.')

assert.equal(densityMass.axisMomentBoundsKgMm2, undefined)
const weightedAxisOptions = { ...planeOptions, inertiaAxis: axisOptions.inertiaAxis }
const weightedAxis = call(request.maxWork, densityOp, weightedAxisOptions)
const expectedWeightedAxisMoment = (2 * 83) / 768 + (6 * 49) / 12288 - 2 * 0.34375 + 2.875
const weightedAxisBounds = weightedAxis.axisMomentBoundsKgMm2
assert.ok(
  weightedAxisBounds &&
    weightedAxisBounds[0] <= expectedWeightedAxisMoment &&
    expectedWeightedAxisMoment <= weightedAxisBounds[1],
)
assert.deepEqual(weightedAxis.centerOfMassPlaneDistanceBoundsMm, centerPlane)
const incompleteWeightedAxis = call(request.maxWork, densityOp, { ...weightedAxisOptions, maxSurfaceCells: 2 })
assert.equal(incompleteWeightedAxis.axisMomentBoundsKgMm2, null)
console.log('WASM ABI enclosed the density-weighted moment about a fixed axis.')

assert.equal(densityMass.referenceInertiaBoundsKgMm2, undefined)
const weightedReferenceOptions = { ...weightedAxisOptions, inertiaReferencePointMm: [1, 0, 0] }
const weightedReference = call(request.maxWork, densityOp, weightedReferenceOptions)
const weightedTensor = weightedReference.referenceInertiaBoundsKgMm2
assert.ok(weightedTensor)
assert.deepEqual(weightedTensor[0][1], weightedTensor[1][0])
assert.ok(weightedTensor[0][1][0] <= 0.375 && 0.375 <= weightedTensor[0][1][1])
assert.ok(
  weightedTensor[2][2][0] <= expectedWeightedAxisMoment && expectedWeightedAxisMoment <= weightedTensor[2][2][1],
)
assert.deepEqual(weightedReference.axisMomentBoundsKgMm2, weightedAxisBounds)
const incompleteWeightedReference = call(request.maxWork, densityOp, {
  ...weightedReferenceOptions,
  maxSurfaceCells: 2,
})
assert.equal(incompleteWeightedReference.referenceInertiaBoundsKgMm2, null)
console.log('WASM ABI enclosed the density-weighted tensor about a fixed point.')

const weightedSpectrumOptions = { ...weightedReferenceOptions, momentToleranceKgMm2: 10 }
const weightedSpectrum = call(request.maxWork, densityOp, weightedSpectrumOptions)
assert.equal(weightedSpectrum.withinMomentTolerance, true)
assert.equal(weightedSpectrum.principalMomentBoundsKgMm2?.length, 3)
assert.deepEqual(weightedSpectrum.referenceInertiaBoundsKgMm2, weightedTensor)
const limitedSpectrum = call(request.maxWork, densityOp, { ...weightedSpectrumOptions, maxSurfaceCells: 2 })
assert.equal(limitedSpectrum.principalMomentBoundsKgMm2, null)
assert.equal(limitedSpectrum.withinMomentTolerance, false)
console.log('WASM ABI enclosed the principal moments of the density-weighted assembly.')

assert.equal(weightedAxis.axisRadiusOfGyrationBoundsMm, undefined)
const weightedRadiusOptions = { ...weightedAxisOptions, includeAxisRadiusOfGyration: true }
const weightedRadius = call(request.maxWork, densityOp, weightedRadiusOptions).axisRadiusOfGyrationBoundsMm
const expectedWeightedRadius = Math.sqrt(expectedWeightedAxisMoment / 2.875)
assert.ok(weightedRadius && weightedRadius[0] <= expectedWeightedRadius && expectedWeightedRadius <= weightedRadius[1])
const limitedWeightedRadius = call(request.maxWork, densityOp, { ...weightedRadiusOptions, maxSurfaceCells: 2 })
assert.equal(limitedWeightedRadius.axisRadiusOfGyrationBoundsMm, null)
console.log('WASM ABI enclosed the radius of gyration of the density-weighted assembly.')

const axisCertificateOptions = {
  ...densityOptions,
  contours: [
    [
      [0.125, 0.125],
      [0.1875, 0.125],
      [0.1875, 0.1875],
      [0.125, 0.1875],
      [0.125, 0.125],
    ],
    [
      [0.125, 0.8125],
      [0.1875, 0.8125],
      [0.1875, 0.875],
      [0.125, 0.875],
      [0.125, 0.8125],
    ],
  ],
  componentDensities: [
    { outerContour: 1, densityBoundsKgPerMm2: [6, 6] },
    { outerContour: 0, densityBoundsKgPerMm2: [2, 2] },
  ],
  includePrincipalAxes: true,
  momentToleranceKgMm2: 1e-8,
  toleranceMm2: 1e-5,
}
const certifiedAxes = call(request.maxWork, densityOp, axisCertificateOptions).principalAxes
assert.ok(certifiedAxes && certifiedAxes.length === 3)
assert.ok(certifiedAxes[0] && certifiedAxes[0].sinAngleUpper < 0.5 && Math.abs(certifiedAxes[0].direction[2]) > 0.9)
assert.deepEqual(certifiedAxes.slice(1), [null, null])
assert.equal(call(request.maxWork, densityOp, { ...axisCertificateOptions, maxSurfaceCells: 1 }).principalAxes, null)
console.log('WASM ABI returned a certified isolated principal direction.')

assert.equal(certifiedAxes[0].distanceBoundsMm, undefined)
const probedAxes = call(request.maxWork, densityOp, {
  ...axisCertificateOptions,
  principalAxisProbePointMm: [1, 0, 1],
}).principalAxes
const axisDistance = probedAxes?.[0]?.distanceBoundsMm
assert.ok(axisDistance && axisDistance[0] <= 0.84375 && 0.84375 <= axisDistance[1])
assert.deepEqual(probedAxes?.slice(1), [null, null])
console.log('WASM ABI enclosed the probe distance to a certified principal axis.')

assert.equal(certifiedAxes[0].angleSineBounds, undefined)
const alignmentAxes = call(request.maxWork, densityOp, {
  ...axisCertificateOptions, principalAxisProbeDirection: [1, 0, 1],
}).principalAxes
const axisAlignment = alignmentAxes?.[0]?.angleSineBounds
assert.ok(axisAlignment && axisAlignment[0] <= Math.SQRT1_2 && Math.SQRT1_2 <= axisAlignment[1])
assert.deepEqual(alignmentAxes?.slice(1), [null, null])
console.log('WASM ABI enclosed the probe angle sine to a certified principal axis.')

assert.equal(weightedAxis.axisBalance, undefined)
const balanceOptions = { ...weightedAxisOptions, axisBalanceToleranceMm: 0.5 }
const balance = call(request.maxWork, densityOp, balanceOptions).axisBalance
const expectedEccentricity = Math.hypot(1 - expectedCenterX, expectedCenterY)
assert.ok(balance && balance.eccentricityBoundsMm[0] <= expectedEccentricity
  && expectedEccentricity <= balance.eccentricityBoundsMm[1])
assert.equal(balance.status, 'outside')
assert.equal(call(request.maxWork, densityOp, { ...balanceOptions, maxSurfaceCells: 2 }).axisBalance, null)
console.log('WASM ABI classified certified eccentricity against the balance tolerance.')

assert.equal(weightedAxis.staticUnbalance, undefined)
const unbalanceOptions = { ...balanceOptions, includeStaticUnbalance: true }
const unbalanceResult = call(request.maxWork, densityOp, unbalanceOptions)
const unbalance = unbalanceResult.staticUnbalance
const expectedUnbalance = [0.34375 - 2.875, 0.375, 0]
assert.ok(unbalance)
for (const [index, exact] of expectedUnbalance.entries()) {
  const [lo, hi] = unbalance.vectorBoundsKgMm[index]
  assert.ok(lo <= exact && exact <= hi)
}
const [unbalanceLo, unbalanceHi] = unbalance.magnitudeBoundsKgMm
assert.ok(unbalanceLo <= Math.hypot(...expectedUnbalance) && Math.hypot(...expectedUnbalance) <= unbalanceHi)
assert.deepEqual(unbalanceResult.axisBalance, balance)
assert.equal(call(request.maxWork, densityOp, { ...unbalanceOptions, maxSurfaceCells: 2 }).staticUnbalance, null)
console.log('WASM ABI enclosed the static unbalance vector and magnitude.')

assert.equal(unbalanceResult.coupleUnbalance, undefined)
const coupleOptions = {
  ...densityOptions,
  inertiaAxis: { pointMm: [0, 0, 0], direction: [0, 0, -2] },
  includeCoupleUnbalance: true,
}
const couple = call(request.maxWork, densityOp, coupleOptions).coupleUnbalance
const expectedCouple = [0.375, -0.34375, 0]
assert.ok(couple)
for (const [index, exact] of expectedCouple.entries()) {
  const [lo, hi] = couple.vectorBoundsKgMm2[index]
  assert.ok(lo <= exact && exact <= hi)
}
const [coupleLo, coupleHi] = couple.magnitudeBoundsKgMm2
assert.ok(coupleLo <= Math.hypot(...expectedCouple) && Math.hypot(...expectedCouple) <= coupleHi)
assert.equal(call(request.maxWork, densityOp, { ...coupleOptions, maxSurfaceCells: 2 }).coupleUnbalance, null)
console.log('WASM ABI enclosed the couple unbalance vector and magnitude.')

const twoPlaneOptions = {
  ...densityOptions,
  inertiaAxis: { pointMm: [1, 0, 0], direction: [0, 0, 1] },
  balancePlaneOffsetsMm: [0, 2],
}
const twoPlane = call(request.maxWork, densityOp, twoPlaneOptions).twoPlaneUnbalance
assert.ok(twoPlane)
assert.deepEqual(twoPlane.map(plane => plane.offsetMm), [0, 2])
for (const plane of twoPlane) {
  for (const [i, exact] of [-1.265625, 0.1875, 0].entries()) {
    const [lo, hi] = plane.vectorBoundsKgMm[i]
    assert.ok(lo <= exact && exact <= hi)
  }
}
assert.equal(call(request.maxWork, densityOp, { ...twoPlaneOptions, maxSurfaceCells: 2 }).twoPlaneUnbalance, null)
console.log('WASM ABI returned equivalent unbalance in both correction planes.')

const planeLimits = { ...twoPlaneOptions, balancePlaneTolerancesKgMm: [5, 0.1] }
const planeBalanceResult = call(request.maxWork, densityOp, planeLimits)
assert.deepEqual(planeBalanceResult.twoPlaneUnbalance, twoPlane)
const planeBalance = planeBalanceResult.twoPlaneBalance
assert.ok(planeBalance)
assert.equal(planeBalance.status, 'outside')
assert.deepEqual(planeBalance.planes.map(plane => [plane.offsetMm, plane.status]), [[0, 'within'], [2, 'outside']])
assert.equal(call(request.maxWork, densityOp, { ...planeLimits, maxSurfaceCells: 2 }).twoPlaneBalance, null)
console.log('WASM ABI classified independent correction-plane tolerances.')

const counterweightOptions = { ...twoPlaneOptions, counterweightRadiiMm: [2, 4] }
const counterweights = call(request.maxWork, densityOp, counterweightOptions).twoPlaneCounterweights
assert.ok(counterweights)
assert.deepEqual(counterweights.map(plane => [plane.offsetMm, plane.radiusMm]), [[0, 2], [2, 4]])
const expectedPlaneMagnitude = Math.hypot(1.265625, 0.1875)
for (const plane of counterweights) {
  const mass = expectedPlaneMagnitude / plane.radiusMm
  assert.ok(plane.massBoundsKg[0] <= mass && mass <= plane.massBoundsKg[1])
  const radial = plane.radialOffsetBoundsMm
  assert.ok(radial)
  for (const [i, component] of [1.265625, -0.1875, 0].entries()) {
    const exact = component * plane.radiusMm / expectedPlaneMagnitude
    assert.ok(radial[i][0] <= exact && exact <= radial[i][1])
  }
}
assert.equal(call(request.maxWork, densityOp, { ...counterweightOptions, maxSurfaceCells: 2 }).twoPlaneCounterweights, null)
console.log('WASM ABI enclosed counterweights at both designated radii.')

const installedWeight = {
  massBoundsKg: [1, 1],
  radialOffsetBoundsMm: [[1.265625, 1.265625], [-0.1875, -0.1875], [0, 0]],
}
const residualOptions = {
  ...twoPlaneOptions, balancePlaneTolerancesKgMm: [5, 5],
  installedCounterweights: [installedWeight, installedWeight],
}
const residualBalance = call(request.maxWork, densityOp, residualOptions).twoPlaneResidualBalance
assert.ok(residualBalance)
assert.equal(residualBalance.status, 'within')
assert.deepEqual(residualBalance.planes.map(plane => plane.offsetMm), [0, 2])
for (const plane of residualBalance.planes) {
  assert.equal(plane.status, 'within')
  assert.ok(plane.vectorBoundsKgMm.every(([lo, hi]) => lo <= 0 && 0 <= hi))
}
assert.equal(call(request.maxWork, densityOp, { ...residualOptions, maxSurfaceCells: 2 }).twoPlaneResidualBalance, null)
console.log('WASM ABI enclosed residual unbalance after installed counterweights.')

const capacityOptions = { ...counterweightOptions, counterweightMassLimitsKg: [5, 0.01] }
const capacityResult = call(request.maxWork, densityOp, capacityOptions)
assert.deepEqual(capacityResult.twoPlaneCounterweights, counterweights)
const capacity = capacityResult.twoPlaneCounterweightCapacity
assert.ok(capacity)
assert.equal(capacity.status, 'outside')
assert.deepEqual(capacity.planes.map(plane => [plane.offsetMm, plane.status]), [[0, 'within'], [2, 'outside']])
for (const [index, weightLimit] of [5, 0.01].entries()) {
  const [lo, hi] = capacity.planes[index].minimumRadiusBoundsMm
  const radius = expectedPlaneMagnitude / weightLimit
  assert.ok(lo <= radius && radius <= hi)
}
assert.equal(call(request.maxWork, densityOp, { ...capacityOptions, maxSurfaceCells: 2 }).twoPlaneCounterweightCapacity, null)
console.log('WASM ABI classified mounting radius capacity under counterweight mass limits.')
