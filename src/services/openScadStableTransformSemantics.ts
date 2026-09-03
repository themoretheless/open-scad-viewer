/**
 * Kernel-neutral transform plans for OpenSCAD 2021.01.
 *
 * Matrices are column-major so they can flow directly into the independent
 * semantic/runtime contracts. Authored multmatrix values remain row-major at
 * the API boundary, matching the language. No safety approximation is made:
 * every plan reports `reduced: false`, while non-finite matrices explicitly
 * report that OpenSCAD removes their child geometry during evaluation.
 */

export type OpenScadStableTransformName =
  | 'translate'
  | 'scale'
  | 'mirror'
  | 'rotate'
  | 'multmatrix'

export type OpenScadStableTransformWarningCode =
  | 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED'
  | 'OPENSCAD_TRANSFORM_PARAMETER_RANGE'
  | 'OPENSCAD_ROTATE_AXIS_IGNORED'
  | 'OPENSCAD_ROTATE_PARTIAL_VECTOR'
  | 'OPENSCAD_TRANSFORM_NONFINITE_EMPTY'

export interface OpenScadStableTransformWarning {
  readonly code: OpenScadStableTransformWarningCode
  readonly message: string
  readonly transform: OpenScadStableTransformName
  readonly field?: string
  readonly value?: unknown
}

export interface OpenScadStableTransformContext {
  readonly warn: (warning: OpenScadStableTransformWarning) => void
  /** Mirrors the optional OpenSCAD parameter-range diagnostic switch. */
  readonly checkParameterRanges?: boolean
}

const SILENT_CONTEXT: OpenScadStableTransformContext = Object.freeze({ warn() {} })

export type OpenScadMatrix4 = readonly [
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
]

export type OpenScadMatrix3 = readonly [
  number, number, number,
  number, number, number,
  number, number, number,
]

export interface OpenScadStableTransformPlan {
  readonly kind: OpenScadStableTransformName
  readonly matrix: OpenScadMatrix4
  /** Full XY/homogeneous submatrix, including an authored projective row. */
  readonly matrix2d: OpenScadMatrix3
  readonly parameterValid: boolean
  /** OpenSCAD discards the transformed node when this is true. */
  readonly dropsChildren: boolean
  readonly matrix3dSingular: boolean
  readonly matrix2dSingular: boolean
  readonly affine3d: boolean
  readonly affine2d: boolean
  /** Pure language normalization never hides an engine approximation. */
  readonly reduced: false
  readonly reduction: null
}

export interface OpenScadRotateInput {
  readonly a?: unknown
  readonly v?: unknown
}

const IDENTITY: OpenScadMatrix4 = Object.freeze([
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
])

function warn(
  context: OpenScadStableTransformContext,
  warning: OpenScadStableTransformWarning,
): void {
  context.warn(Object.freeze(warning))
}

function canonical(value: number): number {
  // Do not round authored matrix/translation/scale values. The degree helpers
  // already return exact quadrant values; only signed zero is observationally
  // irrelevant and awkward for deterministic plans.
  return Object.is(value, -0) ? 0 : value
}

function freezeMatrix4(values: readonly number[]): OpenScadMatrix4 {
  return Object.freeze(values.map(canonical)) as unknown as OpenScadMatrix4
}

function freezeMatrix3(values: readonly number[]): OpenScadMatrix3 {
  return Object.freeze(values.map(canonical)) as unknown as OpenScadMatrix3
}

function exactNumericVector(value: unknown, lengths: readonly number[]): value is number[] {
  return Array.isArray(value)
    && lengths.includes(value.length)
    && value.every(item => typeof item === 'number')
}

interface Vec3Conversion {
  readonly vector: readonly [number, number, number]
  readonly converted: boolean
}

/**
 * Reproduce the 2021 vec3-with-vec2-default conversion, including two useful
 * compatibility details: a malformed vec2 keeps initialized x/y but is still
 * reported converted, while a malformed vec3 can leave a converted prefix.
 */
function vec3WithDefault(
  value: unknown,
  initial: readonly [number, number, number],
  defaultZ: number,
): Vec3Conversion {
  if (!Array.isArray(value)) return Object.freeze({ vector: initial, converted: false })
  if (value.length === 2) {
    if (typeof value[0] === 'number' && typeof value[1] === 'number') {
      return Object.freeze({ vector: [value[0], value[1], defaultZ] as const, converted: true })
    }
    return Object.freeze({ vector: [initial[0], initial[1], defaultZ] as const, converted: true })
  }
  if (value.length !== 3) return Object.freeze({ vector: initial, converted: false })
  const vector: [number, number, number] = [...initial]
  for (let index = 0; index < 3; index++) {
    if (typeof value[index] !== 'number') return Object.freeze({ vector, converted: false })
    vector[index] = value[index]
  }
  return Object.freeze({ vector, converted: true })
}

function sinDegrees(value: number): number {
  if (!Number.isFinite(value)) return Number.NaN
  const quadrant = value / 90
  if (Number.isInteger(quadrant)) return [0, 1, 0, -1][((quadrant % 4) + 4) % 4]
  return Math.sin(value * Math.PI / 180)
}

function cosDegrees(value: number): number {
  if (!Number.isFinite(value)) return Number.NaN
  const quadrant = value / 90
  if (Number.isInteger(quadrant)) return [1, 0, -1, 0][((quadrant % 4) + 4) % 4]
  return Math.cos(value * Math.PI / 180)
}

function matrixFromRows(rows: readonly (readonly number[])[]): OpenScadMatrix4 {
  const values: number[] = []
  for (let column = 0; column < 4; column++) {
    for (let row = 0; row < 4; row++) values.push(rows[row][column])
  }
  return freezeMatrix4(values)
}

/** Extract OpenSCAD's complete XY/homogeneous transform from a 4x4 matrix. */
export function openScadTransform2dSubmatrix(matrix: OpenScadMatrix4): OpenScadMatrix3 {
  return freezeMatrix3([
    matrix[0], matrix[1], matrix[3],
    matrix[4], matrix[5], matrix[7],
    matrix[12], matrix[13], matrix[15],
  ])
}

function determinant(values: readonly number[], size: 3 | 4): number {
  const rows = Array.from({ length: size }, (_, row) =>
    Array.from({ length: size }, (_, column) => values[column * size + row]))
  let sign = 1
  let result = 1
  for (let column = 0; column < size; column++) {
    let pivot = column
    while (pivot < size && rows[pivot][column] === 0) pivot++
    if (pivot === size) return 0
    if (pivot !== column) {
      const swap = rows[column]
      rows[column] = rows[pivot]
      rows[pivot] = swap
      sign *= -1
    }
    const pivotValue = rows[column][column]
    result *= pivotValue
    for (let row = column + 1; row < size; row++) {
      const factor = rows[row][column] / pivotValue
      for (let inner = column + 1; inner < size; inner++) {
        rows[row][inner] -= factor * rows[column][inner]
      }
    }
  }
  return sign * result
}

function finish(
  kind: OpenScadStableTransformName,
  matrix: OpenScadMatrix4,
  parameterValid: boolean,
  context: OpenScadStableTransformContext,
): OpenScadStableTransformPlan {
  const matrix2d = openScadTransform2dSubmatrix(matrix)
  const finite3d = matrix.every(Number.isFinite)
  const finite2d = matrix2d.every(Number.isFinite)
  if (!finite3d) warn(context, {
    code: 'OPENSCAD_TRANSFORM_NONFINITE_EMPTY',
    message: `${kind} produced a non-finite matrix, so its child geometry is removed`,
    transform: kind,
  })
  return Object.freeze({
    kind,
    matrix,
    matrix2d,
    parameterValid,
    dropsChildren: !finite3d,
    matrix3dSingular: finite3d && determinant(matrix, 4) === 0,
    matrix2dSingular: finite2d && determinant(matrix2d, 3) === 0,
    affine3d: matrix[3] === 0 && matrix[7] === 0 && matrix[11] === 0 && matrix[15] === 1,
    affine2d: matrix2d[2] === 0 && matrix2d[5] === 0 && matrix2d[8] === 1,
    reduced: false,
    reduction: null,
  })
}

export function resolveOpenScadTranslate(
  value: unknown,
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  const converted = vec3WithDefault(value, [0, 0, 0], 0)
  const valid = converted.converted && converted.vector.every(Number.isFinite)
  const vector = valid ? converted.vector : [0, 0, 0]
  if (!valid) warn(context, {
    code: 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    message: 'translate uses identity because v is not a finite exact vec2 or vec3',
    transform: 'translate',
    field: 'v',
    value,
  })
  const matrix = [...IDENTITY] as number[]
  matrix[12] = vector[0]
  matrix[13] = vector[1]
  matrix[14] = vector[2]
  return finish('translate', freezeMatrix4(matrix), valid, context)
}

export function resolveOpenScadScale(
  value: unknown,
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  const converted = vec3WithDefault(value, [1, 1, 1], 1)
  let vector: readonly number[] = converted.vector
  let valid = converted.converted
  if (!valid && typeof value === 'number') {
    vector = [value, value, value]
    valid = true
  } else if (!valid) {
    warn(context, {
      code: 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
      message: 'scale uses identity because v is not a scalar or exact vec2/vec3',
      transform: 'scale',
      field: 'v',
      value,
    })
  }
  if (context.checkParameterRanges && vector.some(component => component === 0 || !Number.isFinite(component))) {
    warn(context, {
      code: 'OPENSCAD_TRANSFORM_PARAMETER_RANGE',
      message: 'scale contains a zero or non-finite component',
      transform: 'scale',
      field: 'v',
      value,
    })
  }
  return finish('scale', freezeMatrix4([
    vector[0], 0, 0, 0,
    0, vector[1], 0, 0,
    0, 0, vector[2], 0,
    0, 0, 0, 1,
  ]), valid, context)
}

function mirrorMatrix(vector: readonly [number, number, number]): OpenScadMatrix4 {
  const [x, y, z] = vector
  if (x === 0 && y === 0 && z === 0) return IDENTITY
  const magnitudeSquared = x * x + y * y + z * z
  return matrixFromRows([
    [1 - 2 * x * x / magnitudeSquared, -2 * y * x / magnitudeSquared, -2 * z * x / magnitudeSquared, 0],
    [-2 * x * y / magnitudeSquared, 1 - 2 * y * y / magnitudeSquared, -2 * z * y / magnitudeSquared, 0],
    [-2 * x * z / magnitudeSquared, -2 * y * z / magnitudeSquared, 1 - 2 * z * z / magnitudeSquared, 0],
    [0, 0, 0, 1],
  ])
}

export function resolveOpenScadMirror(
  value: unknown,
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  const converted = vec3WithDefault(value, [1, 0, 0], 0)
  if (!converted.converted) warn(context, {
    code: 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    message: 'mirror uses its x-normal default because v is not an exact vec2 or vec3',
    transform: 'mirror',
    field: 'v',
    value,
  })
  return finish('mirror', mirrorMatrix(converted.vector), converted.converted, context)
}

function axisAngleMatrix(degrees: number, axis: readonly [number, number, number]): OpenScadMatrix4 {
  const [vx, vy, vz] = axis
  const magnitude = Math.hypot(vx, vy, vz)
  if (!(magnitude > 0)) return IDENTITY
  const x = vx / magnitude
  const y = vy / magnitude
  const z = vz / magnitude
  const sine = sinDegrees(degrees)
  const cosine = cosDegrees(degrees)
  const complement = 1 - cosine
  return matrixFromRows([
    [complement * x * x + cosine, complement * x * y - sine * z, complement * x * z + sine * y, 0],
    [complement * x * y + sine * z, complement * y * y + cosine, complement * y * z - sine * x, 0],
    [complement * x * z - sine * y, complement * y * z + sine * x, complement * z * z + cosine, 0],
    [0, 0, 0, 1],
  ])
}

function eulerMatrix(x: number, y: number, z: number): OpenScadMatrix4 {
  const sx = sinDegrees(x), cx = cosDegrees(x)
  const sy = sinDegrees(y), cy = cosDegrees(y)
  const sz = sinDegrees(z), cz = cosDegrees(z)
  return matrixFromRows([
    [cy * cz, cz * sx * sy - cx * sz, cx * cz * sy + sx * sz, 0],
    [cy * sz, cx * cz + sx * sy * sz, -cz * sx + cx * sy * sz, 0],
    [-sy, cy * sx, cx * cy, 0],
    [0, 0, 0, 1],
  ])
}

function vectorEulerAngles(value: readonly unknown[]): {
  readonly x: number
  readonly y: number
  readonly z: number
  readonly valid: boolean
} {
  let remembered = 0
  let valid = value.length <= 3
  const component = (index: number): number => {
    if (typeof value[index] === 'number') remembered = value[index]
    else valid = false
    if (!Number.isFinite(remembered)) valid = false
    return remembered
  }
  const z = value.length >= 3 ? component(2) : 0
  const y = value.length >= 2 ? component(1) : 0
  const x = value.length >= 1 ? component(0) : 0
  return Object.freeze({ x, y, z, valid })
}

export function resolveOpenScadRotate(
  input: OpenScadRotateInput = {},
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  if (Array.isArray(input.a)) {
    const angles = vectorEulerAngles(input.a)
    if (angles.valid && input.v !== undefined) warn(context, {
      code: 'OPENSCAD_ROTATE_AXIS_IGNORED',
      message: 'rotate ignores v when a is a vector',
      transform: 'rotate',
      field: 'v',
      value: input.v,
    })
    if (!angles.valid) warn(context, {
      code: 'OPENSCAD_ROTATE_PARTIAL_VECTOR',
      message: 'rotate retained its component-wise fallback matrix after a vector conversion problem',
      transform: 'rotate',
      field: 'a',
      value: input.a,
    })
    return finish('rotate', eulerMatrix(angles.x, angles.y, angles.z), angles.valid, context)
  }

  const angleValid = typeof input.a === 'number' && Number.isFinite(input.a)
  const degrees = angleValid ? input.a as number : 0
  const axisProvided = input.v !== undefined
  const convertedAxis = axisProvided
    ? vec3WithDefault(input.v, [0, 0, 1], 0)
    : Object.freeze({ vector: [0, 0, 1] as const, converted: true })
  const axisValid = convertedAxis.converted
  if (!angleValid || !axisValid) warn(context, {
    code: 'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    message: 'rotate replaced an invalid scalar angle or axis with its neutral/default value',
    transform: 'rotate',
    field: !angleValid ? 'a' : 'v',
    value: !angleValid ? input.a : input.v,
  })
  return finish('rotate', axisAngleMatrix(degrees, convertedAxis.vector), angleValid && axisValid, context)
}

export function resolveOpenScadMultmatrix(
  value: unknown,
  context: OpenScadStableTransformContext = SILENT_CONTEXT,
): OpenScadStableTransformPlan {
  if (!Array.isArray(value)) return finish('multmatrix', IDENTITY, false, context)
  const rows = [
    [1, 0, 0, 0],
    [0, 1, 0, 0],
    [0, 0, 1, 0],
    [0, 0, 0, 1],
  ]
  for (let row = 0; row < Math.min(4, value.length); row++) {
    const authoredRow = Array.isArray(value[row]) ? value[row] : []
    for (let column = 0; column < Math.min(4, authoredRow.length); column++) {
      const authoredCell = authoredRow[column]
      if (typeof authoredCell === 'number') rows[row][column] = authoredCell
    }
  }
  const w = rows[3][3]
  if (w !== 1) {
    for (let row = 0; row < 4; row++) {
      for (let column = 0; column < 4; column++) rows[row][column] /= w
    }
  }
  return finish('multmatrix', matrixFromRows(rows), true, context)
}
