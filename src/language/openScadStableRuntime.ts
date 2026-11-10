/**
 * Host-supplied special variables for the repository-owned OpenSCAD 2021.01
 * evaluator. Geometry engines share this initializer so `$t`, `$preview`, and
 * the default camera never drift between execution lanes.
 */

export interface OpenScadStableRuntimeOptions {
  readonly quality: 'preview' | 'full'
  readonly animationTime?: number
}

export type OpenScadStableRuntimeValue = number | boolean | readonly number[]

export const OPENSCAD_2021_DEFAULT_VIEW = Object.freeze({
  translation: Object.freeze([0, 0, 0] as const),
  rotation: Object.freeze([55, 0, 25] as const),
  distance: 140,
  fieldOfView: 22.5,
})

/**
 * Return a fresh map because OpenSCAD `$` variables are dynamically scoped
 * and an authored assignment may shadow a host value during evaluation.
 */
export function createOpenScadStableRuntimeVariables(
  options: OpenScadStableRuntimeOptions,
): Map<string, OpenScadStableRuntimeValue> {
  const animationTime = options.animationTime ?? 0
  if (!Number.isFinite(animationTime) || animationTime < 0 || animationTime > 1) {
    throw new RangeError('OpenSCAD animationTime must be a finite number between 0 and 1')
  }
  return new Map<string, OpenScadStableRuntimeValue>([
    ['$fn', 0],
    ['$fa', 12],
    ['$fs', 2],
    ['$t', animationTime],
    ['$preview', options.quality === 'preview'],
    ['$vpt', [...OPENSCAD_2021_DEFAULT_VIEW.translation]],
    ['$vpr', [...OPENSCAD_2021_DEFAULT_VIEW.rotation]],
    ['$vpd', OPENSCAD_2021_DEFAULT_VIEW.distance],
    ['$vpf', OPENSCAD_2021_DEFAULT_VIEW.fieldOfView],
  ])
}
