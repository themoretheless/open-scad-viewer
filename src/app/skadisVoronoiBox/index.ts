import { boxGeometrySource } from './legacyBoxSource'
import {
  BOX_DEFAULTS,
  isSkadisMount,
  SKADIS_CONSTANTS,
  SKADIS_VORONOI_BOX_DEFAULTS,
  type FloorLatticeMode,
  type LatticeMode,
  type LegacyBoxParams,
  type MountSystem,
  type SkadisLayoutInfo,
  type SkadisVoronoiBoxParams,
} from './schema'
import { SKADIS_VORONOI_PRESETS, type SkadisVoronoiPreset } from './presets'
import { emitSkadisVoronoiScad } from './skadisScad'
import { computeSkadisLayout, parseLegacyBox, parseSkadisVoronoiBox, summarizeSkadisBox } from './validation'

export {
  BOX_DEFAULTS,
  computeSkadisLayout,
  isSkadisMount,
  parseLegacyBox,
  parseSkadisVoronoiBox,
  SKADIS_CONSTANTS,
  SKADIS_VORONOI_BOX_DEFAULTS,
  SKADIS_VORONOI_PRESETS,
  summarizeSkadisBox,
  type FloorLatticeMode,
  type LatticeMode,
  type LegacyBoxParams,
  type MountSystem,
  type SkadisLayoutInfo,
  type SkadisVoronoiBoxParams,
  type SkadisVoronoiPreset,
}

function formatLegacyBoxScad(p: LegacyBoxParams): string {
  return `// Organizer box. External dimensions in mm; organic skeletal walls.
// Generic rail hooks: measure your rail. No load rating or physical fit test.
${Object.entries(p)
  .map(([key, value]) => `${key} = ${value};`)
  .join('\n')}
${boxGeometrySource}`
}

/** Generate OpenSCAD source for the unified SKÅDIS & Voronoi box mechanism. */
export function generateSkadisVoronoiBox(
  input: unknown = SKADIS_VORONOI_BOX_DEFAULTS,
  locale: 'ru' | 'en' = 'ru',
): string {
  const p = parseSkadisVoronoiBox(input)
  return emitSkadisVoronoiScad(p, locale)
}

/** Backward-compatible entry point used by legacy organizer box tests and ?generator=box. */
export function generateBox(input: unknown = BOX_DEFAULTS): string {
  const { legacy } = parseLegacyBox(input)
  return formatLegacyBoxScad(legacy)
}
