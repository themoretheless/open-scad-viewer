// Frozen TS comparison oracle; production calls the native OpenSCAD kernel.
type OpenScadPoint2 = readonly [number, number]
type OpenScadScale2 = readonly [number, number]
type OpenScadLinearExtrudeSliceSource = string
const OPENSCAD_2021_GEOMETRY_EPSILON = 0.00000095367431640625
function farthestRadiusSquared(points: readonly OpenScadPoint2[]): number {
  let maximum = 0
  for (const [x, y] of points) maximum = Math.max(maximum, x * x + y * y)
  return maximum
}

function farthestScaleDeltaSquared(
  points: readonly OpenScadPoint2[],
  scale: OpenScadScale2,
): number {
  let maximum = 0
  for (const [x, y] of points) {
    const dx = x * (1 - scale[0])
    const dy = y * (1 - scale[1])
    maximum = Math.max(maximum, dx * dx + dy * dy)
  }
  return maximum
}

function spiralPlanarLength(radius: number, radians: number, scale: number): number {
  const radialSlope = radius * (scale - 1) / radians
  const primitive = (radial: number): number => {
    const slopeMagnitude = Math.abs(radialSlope)
    return 0.5 * (
      radial * Math.hypot(radial, radialSlope)
      + radialSlope * radialSlope * Math.asinh(radial / slopeMagnitude)
    )
  }
  return Math.abs((primitive(radius * scale) - primitive(radius)) / radialSlope)
}

export function automaticTwistSlices(
  points: readonly OpenScadPoint2[],
  height: number,
  twist: number,
  scale: OpenScadScale2,
  fn: number,
  fa: number,
  fs: number,
): { slices: number; source: OpenScadLinearExtrudeSliceSource } {
  const absoluteTwist = Math.abs(twist)
  const minimum = Math.max(1, Math.ceil(absoluteTwist / 120))
  const radius = Math.sqrt(farthestRadiusSquared(points))
  if (radius < OPENSCAD_2021_GEOMETRY_EPSILON || !Number.isFinite(fn)) {
    return { slices: minimum, source: 'twist-minimum' }
  }
  if (fn > 0) {
    return {
      slices: Math.max(minimum, Math.ceil(absoluteTwist * fn / 360)),
      source: 'twist-$fn',
    }
  }

  const radians = absoluteTwist * Math.PI / 180
  const planarLength = scale[0] === scale[1] && scale[0] !== 1
    ? spiralPlanarLength(radius, radians, scale[0])
    : radius * radians
  const pathLength = Math.hypot(planarLength, height)
  const angularSlices = Math.ceil(absoluteTwist / fa)
  const lengthSlices = Math.ceil(pathLength / fs)
  return {
    slices: Math.max(minimum, Math.min(angularSlices, lengthSlices)),
    source: 'twist-$fa/$fs',
  }
}

export function automaticNonuniformSlices(
  points: readonly OpenScadPoint2[],
  height: number,
  scale: OpenScadScale2,
  fn: number,
  fs: number,
): { slices: number; source: OpenScadLinearExtrudeSliceSource } {
  const displacement = Math.sqrt(farthestScaleDeltaSquared(points, scale))
  if (displacement < OPENSCAD_2021_GEOMETRY_EPSILON || !Number.isFinite(fn)) {
    return { slices: 1, source: 'single' }
  }
  if (fn > 0) {
    return { slices: Math.max(1, Math.trunc(fn)), source: 'nonuniform-$fn' }
  }
  return {
    slices: Math.max(1, Math.ceil(Math.hypot(displacement, height) / fs)),
    source: 'nonuniform-$fs',
  }
}

