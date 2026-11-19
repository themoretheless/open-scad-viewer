/**
 * OpenSCAD evaluator warnings. Both the TypeScript evaluator and the native Rust evaluator
 * report them as plain English strings over the worker protocol, so the code is recovered
 * from the text: each pattern matches every message its code is emitted with.
 */
export const OPENSCAD_WARNING_PATTERNS = [
  ['OPENSCAD_CHILDREN_BAD_INDEX', /^Non-numeric children index was ignored$/],
  ['OPENSCAD_CHILDREN_BAD_SELECTION', /^(Invalid children range was ignored|children accepts an empty argument list)/],
  ['OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS', /^Children index \S+ is outside 0\.\./],
  ['OPENSCAD_CHILDREN_RANGE_LIMIT', /^children range exceeds the engine limit/],
  ['OPENSCAD_COLOR_ALPHA_RANGE', /^The color vector alpha channel is outside/],
  ['OPENSCAD_COLOR_CHANNEL_RANGE', /^Color channel \S+ is outside the inclusive 0\.\.1 range$/],
  ['OPENSCAD_COLOR_UNKNOWN', /^Unknown OpenSCAD color name: /],
  ['OPENSCAD_CYLINDER_AMBIGUOUS_RADII', /^cylinder combines a shared radius with an end-specific radius$/],
  ['OPENSCAD_FA_TOO_SMALL', /^\$fa was raised to the OpenSCAD minimum /],
  ['OPENSCAD_FS_TOO_SMALL', /^\$fs was raised to the OpenSCAD minimum /],
  ['OPENSCAD_FN_NEGATIVE', /^Negative \$fn was replaced/],
  ['OPENSCAD_FRAGMENTS_CLAMPED', /^Fragment count was clamped to the engine safety limit/],
  [
    'OPENSCAD_FRAGMENT_NON_FINITE',
    /^Non-finite (\$fa was replaced|\$fs was replaced|\$fn selected|fragment resolution was bounded)/,
  ],
  ['OPENSCAD_LINEAR_EXTRUDE_HEIGHT_DEFAULTED', /^Invalid linear_extrude height was replaced/],
  ['OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED', /^Invalid linear_extrude scale was replaced/],
  ['OPENSCAD_LINEAR_EXTRUDE_SLICES_CLAMPED', /^linear_extrude slices were clamped/],
  ['OPENSCAD_POLYHEDRON_TRIANGLES_DEPRECATED', /^polyhedron triangles is a legacy alias/],
  [
    'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS',
    /^poly(gon|hedron) skipped a (path|face) entry whose point index is outside/,
  ],
  ['OPENSCAD_PRIMITIVE_POINT_INVALID', /^poly(gon produced no outlines|hedron stopped) .*failed exact vec/],
  ['OPENSCAD_PRIMITIVE_RADIUS_SHADOWED', /^\w+ uses \w+; the paired \w+ value has no effect$/],
  ['OPENSCAD_PRIMITIVE_RANGE_EMPTY', /^\w+ parameters describe an empty object$/],
  ['OPENSCAD_PRIMITIVE_SAFETY_REDUCTION', /^\w+ was not expanded because its .* exceed the configured engine limit$/],
  ['OPENSCAD_PRIMITIVE_SIZE_DEFAULTED', /^\w+ size was not a scalar or exact .*; unit size is used$/],
  ['OPENSCAD_RESIZE_BAD_NEWSIZE', /^resize newsize must be a vector$/],
  ['OPENSCAD_RESIZE_ZERO_EXTENT', /^resize cannot map a zero-width axis/],
  ['OPENSCAD_ROTATE_AXIS_IGNORED', /^rotate ignores v when a is a vector$/],
  ['OPENSCAD_ROTATE_EXTRUDE_ANGLE_DEFAULTED', /^Invalid rotate_extrude angle was replaced/],
  ['OPENSCAD_ROTATE_PARTIAL_VECTOR', /^rotate retained its component-wise fallback matrix/],
  ['OPENSCAD_TRANSFORM_NONFINITE_EMPTY', /^\w+ produced a non-finite matrix, so its child geometry is removed$/],
  [
    'OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED',
    /^(mirror uses its x-normal default|rotate replaced an invalid scalar|scale uses identity because|translate uses identity because)/,
  ],
  ['OPENSCAD_TRANSFORM_PARAMETER_RANGE', /^scale contains a zero or non-finite component$/],
  ['OPENSCAD_DXF_QUERY_INVALID_ORIGIN', /^DXF query origin /],
  [
    'OPENSCAD_DXF_QUERY_FILE_UNAVAILABLE',
    /^DXF query (path could not be resolved|asset .* is missing from the project)/,
  ],
  ['OPENSCAD_DXF_QUERY_NOT_PREPARED', /^DXF query asset .* was not prepared as DXF\.$/],
  ['OPENSCAD_DXF_QUERY_INVALID_DATA', /^DXF query asset .* could not be decoded/],
  ['OPENSCAD_DXF_DIMENSION_NOT_FOUND', /^No matching DXF dimension was found/],
  ['OPENSCAD_DXF_DIMENSION_TYPE_UNSUPPORTED', /^DXF dimension .* uses unsupported type/],
  ['OPENSCAD_DXF_CROSS_NOT_FOUND', /^The first two eligible DXF paths .* do not define a cross\.$/],
] as const satisfies readonly (readonly [string, RegExp])[]

/**
 * rotate_extrude re-labels the fragment warnings without changing their text, so these
 * codes are indistinguishable from their targets once the warning is a string.
 */
export const OPENSCAD_WARNING_ALIASES = {
  OPENSCAD_ROTATE_EXTRUDE_FRAGMENTS_CLAMPED: 'OPENSCAD_FRAGMENTS_CLAMPED',
  OPENSCAD_ROTATE_EXTRUDE_FRAGMENT_PARAMETER: 'OPENSCAD_FRAGMENT_NON_FINITE',
} as const

export type OpenScadWarningCode = (typeof OPENSCAD_WARNING_PATTERNS)[number][0]

export const OPENSCAD_WARNING_REASONS = [
  'color',
  'fragments',
  'children',
  'argumentDefaulted',
  'engineLimit',
  'primitive',
  'resize',
  'deprecated',
  'ignoredArgument',
  'dxfQuery',
] as const
export type OpenScadWarningReason = (typeof OPENSCAD_WARNING_REASONS)[number]

const OPENSCAD_WARNING_REASON: Record<OpenScadWarningCode, OpenScadWarningReason> = {
  OPENSCAD_CHILDREN_BAD_INDEX: 'children',
  OPENSCAD_CHILDREN_BAD_SELECTION: 'children',
  OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS: 'children',
  OPENSCAD_CHILDREN_RANGE_LIMIT: 'engineLimit',
  OPENSCAD_COLOR_ALPHA_RANGE: 'color',
  OPENSCAD_COLOR_CHANNEL_RANGE: 'color',
  OPENSCAD_COLOR_UNKNOWN: 'color',
  OPENSCAD_CYLINDER_AMBIGUOUS_RADII: 'ignoredArgument',
  OPENSCAD_FA_TOO_SMALL: 'fragments',
  OPENSCAD_FS_TOO_SMALL: 'fragments',
  OPENSCAD_FN_NEGATIVE: 'fragments',
  OPENSCAD_FRAGMENTS_CLAMPED: 'engineLimit',
  OPENSCAD_FRAGMENT_NON_FINITE: 'fragments',
  OPENSCAD_LINEAR_EXTRUDE_HEIGHT_DEFAULTED: 'argumentDefaulted',
  OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED: 'argumentDefaulted',
  OPENSCAD_LINEAR_EXTRUDE_SLICES_CLAMPED: 'engineLimit',
  OPENSCAD_POLYHEDRON_TRIANGLES_DEPRECATED: 'deprecated',
  OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS: 'primitive',
  OPENSCAD_PRIMITIVE_POINT_INVALID: 'primitive',
  OPENSCAD_PRIMITIVE_RADIUS_SHADOWED: 'ignoredArgument',
  OPENSCAD_PRIMITIVE_RANGE_EMPTY: 'primitive',
  OPENSCAD_PRIMITIVE_SAFETY_REDUCTION: 'engineLimit',
  OPENSCAD_PRIMITIVE_SIZE_DEFAULTED: 'argumentDefaulted',
  OPENSCAD_RESIZE_BAD_NEWSIZE: 'resize',
  OPENSCAD_RESIZE_ZERO_EXTENT: 'resize',
  OPENSCAD_ROTATE_AXIS_IGNORED: 'ignoredArgument',
  OPENSCAD_ROTATE_EXTRUDE_ANGLE_DEFAULTED: 'argumentDefaulted',
  OPENSCAD_ROTATE_PARTIAL_VECTOR: 'argumentDefaulted',
  OPENSCAD_TRANSFORM_NONFINITE_EMPTY: 'argumentDefaulted',
  OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED: 'argumentDefaulted',
  OPENSCAD_TRANSFORM_PARAMETER_RANGE: 'argumentDefaulted',
  OPENSCAD_DXF_QUERY_INVALID_ORIGIN: 'dxfQuery',
  OPENSCAD_DXF_QUERY_FILE_UNAVAILABLE: 'dxfQuery',
  OPENSCAD_DXF_QUERY_NOT_PREPARED: 'dxfQuery',
  OPENSCAD_DXF_QUERY_INVALID_DATA: 'dxfQuery',
  OPENSCAD_DXF_DIMENSION_NOT_FOUND: 'dxfQuery',
  OPENSCAD_DXF_DIMENSION_TYPE_UNSUPPORTED: 'dxfQuery',
  OPENSCAD_DXF_CROSS_NOT_FOUND: 'dxfQuery',
}

/** Code of a known OpenSCAD evaluator warning, or `undefined` for other text (ECHO, etc.). */
export function openScadWarningCode(warning: string): OpenScadWarningCode | undefined {
  return OPENSCAD_WARNING_PATTERNS.find(([, pattern]) => pattern.test(warning))?.[0]
}

export function openScadWarningReason(code: string): OpenScadWarningReason | undefined {
  const target = Object.hasOwn(OPENSCAD_WARNING_ALIASES, code)
    ? OPENSCAD_WARNING_ALIASES[code as keyof typeof OPENSCAD_WARNING_ALIASES]
    : code
  return Object.hasOwn(OPENSCAD_WARNING_REASON, target)
    ? OPENSCAD_WARNING_REASON[target as OpenScadWarningCode]
    : undefined
}

/** Localized warning line: reason and next step, the code, and the evaluator text as detail. */
export function presentBuildWarning(warning: string, t: (key: string) => string): string {
  const code = openScadWarningCode(warning)
  if (!code) return warning
  return `${t(`openScadWarning.${OPENSCAD_WARNING_REASON[code]}`)} (${code}) ${warning}`
}
