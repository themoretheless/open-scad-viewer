/** Public polygon API. No spline representation or Manifold dependency. */
import { callGeometryRust } from './kernel'
export interface PolygonMesh {
  positions: number[]
  indices: number[]
  uv?: number[]
}
export interface PolygonReport {
  triangleCount: number
  vertexCount: number
  boundaryEdges: number
  nonManifoldEdges: number
  orientationConflicts: number
  degenerateTriangles: number
  closed: boolean
  signedVolumeMm3: number
  errorBoundCertified: false
  selfIntersectionStatus: 'not_checked' | 'checked_with_tolerance'
  boolean?: PolygonBooleanReport
  construction: 'boolean' | 'triangle_mesh' | 'sampled_surface' | 'fixed_vector_thickening'
  uvArea?: number
  sampledDeviationMm?: number
  parameterSeamsWelded?: { u: boolean; v: boolean }
  collapsedBoundaryCount?: number
}
export interface PolygonBuild extends PolygonMesh { report: PolygonReport }
export function inspectPolygonMesh(mesh: PolygonMesh): PolygonReport { return callGeometryRust('mesh_inspect', { mesh }) }
export function polygonBoundaryLoops(mesh: PolygonMesh): number[][] { return callGeometryRust('mesh_boundary_loops', { mesh }) }
export function transformPolygonMesh(mesh: PolygonMesh, matrix: number[][]): PolygonBuild { return callGeometryRust('mesh_transform', { mesh, matrix }) }
export function thickenPolygonMesh(mesh: PolygonMesh, vector: number[]): PolygonBuild { return callGeometryRust('mesh_thicken', { mesh, vector }) }
export function exportPolygonStl(mesh: PolygonMesh): string { return callGeometryRust('mesh_export_stl', { mesh }) }

export type PolygonBooleanOperation = 'union' | 'intersection' | 'difference'
export interface PolygonBooleanOptions {
  relativeTolerance?: number
  maxWork?: number
  maxFragments?: number
  maxOutputTriangles?: number
}
export interface PolygonBooleanReport {
  operation: PolygonBooleanOperation
  toleranceMm: number
  work: number
  fragments: number
  inputTriangles: [number, number]
}
export function booleanPolygonMeshes(a: PolygonMesh, b: PolygonMesh, operation: PolygonBooleanOperation, options: PolygonBooleanOptions = {}): PolygonBuild {
  return callGeometryRust('mesh_boolean', { a, b, operation, options })
}

/** Horizontal mesh section at z (mm). Contours are not yet printable regions. */
export interface MeshSectionContour {
  points: number[][]
  sourceTriangles: number[]
}
export interface MeshSectionResult {
  z_mm: number
  candidateTriangles: number
  contours: MeshSectionContour[]
}
export function sectionPolygonMesh(mesh: PolygonMesh, z: number): MeshSectionResult {
  return callGeometryRust('mesh_section', { mesh, z })
}

export interface ToolpathSettingsInput {
  layerHeightMm?: number
  lineWidthMm?: number
  wallCount?: number
  infillSpacingMm?: number
  feedrateMmS?: number
  travelFeedrateMmS?: number
  filamentDiameterMm?: number
}
export interface ToolpathPath {
  role: 'outline' | 'inset' | 'hatch'
  closed: boolean
  points: number[][]
}
export interface ToolpathLayerResult {
  z_mm: number
  paths: ToolpathPath[]
}
export interface ToolpathPlanResult {
  layers: ToolpathLayerResult[]
}
export interface GcodePreviewMove {
  x: number
  y: number
  z: number
  /** Absolute filament length in mm. */
  e: number
  extruded: boolean
  feedrateMmS: number
  layerIndex: number
}
export const GCODE_PREVIEW_DIALECT = 'open-scad-viewer/print-preview 2'
export const GCODE_JOB_DIALECT = 'open-scad-viewer/print-job 1'
/** Firmware families for `mesh_gcode_job`; names follow slicer `gcode_flavor` values. */
export const GCODE_FLAVORS = ['marlin', 'klipper', 'reprapfirmware'] as const
export type GcodeFlavor = typeof GCODE_FLAVORS[number]
export interface GcodePreviewResult {
  layers: number
  extrusionMm: number
  depositedVolumeMm3: number
  bounds: { min: [number, number, number]; max: [number, number, number] } | null
  travelDistanceMm: number
  printDistanceMm: number
  /** Constant-speed estimate; excludes initial positioning and firmware dynamics. */
  estimatedTimeS: number
  moves: GcodePreviewMove[]
}
export interface GcodeExportResult {
  gcode: string
  dialect: string
  layerCount: number
  preview: GcodePreviewResult
}
export interface GcodeJobExportResult extends GcodeExportResult {
  /** Stored OPC `.gcode.3mf` bytes, standard base64. */
  gcode3mfBase64: string
  flavor: GcodeFlavor
}
/** Result of opening an arbitrary G-code file: preview plus what was detected. */
export interface GcodeInspectResult {
  preview: GcodePreviewResult
  /** Native dialect line, or `"<generator> G-code (tolerant preview)"` for foreign files. */
  dialect: string
  /** True for this app's own strict preview/job dialects. */
  native: boolean
  generator: string
  /** Firmware flavor declared by the slicer header, when present. */
  flavor: string | null
  filamentDiameterMm: number | null
}
export interface JobSettingsInput extends ToolpathSettingsInput {
  nozzleTempC?: number
  bedTempC?: number
  retractLengthMm?: number
  retractFeedrateMmS?: number
  unretractFeedrateMmS?: number
  retractMinTravelMm?: number
  fanSpeed?: number
  homeAxes?: boolean
  flavor?: GcodeFlavor
  simplifyToleranceMm?: number
  max2optSwaps?: number
}

// Explicit fields preserve the operation and mesh even for untyped callers.
function toolpathArguments(settings: ToolpathSettingsInput): ToolpathSettingsInput {
  return {
    layerHeightMm: settings.layerHeightMm,
    lineWidthMm: settings.lineWidthMm,
    wallCount: settings.wallCount,
    infillSpacingMm: settings.infillSpacingMm,
    feedrateMmS: settings.feedrateMmS,
    travelFeedrateMmS: settings.travelFeedrateMmS,
    filamentDiameterMm: settings.filamentDiameterMm,
  }
}
function jobArguments(settings: JobSettingsInput): JobSettingsInput {
  return {
    ...toolpathArguments(settings),
    nozzleTempC: settings.nozzleTempC,
    bedTempC: settings.bedTempC,
    retractLengthMm: settings.retractLengthMm,
    retractFeedrateMmS: settings.retractFeedrateMmS,
    unretractFeedrateMmS: settings.unretractFeedrateMmS,
    retractMinTravelMm: settings.retractMinTravelMm,
    fanSpeed: settings.fanSpeed,
    homeAxes: settings.homeAxes,
    flavor: settings.flavor,
    simplifyToleranceMm: settings.simplifyToleranceMm,
    max2optSwaps: settings.max2optSwaps,
  }
}
/** Samples Z = zMin + i * layerHeightMm below zMax, preserving model coordinates. */
export function planPolygonMeshToolpaths(
  mesh: PolygonMesh,
  zMin: number,
  zMax: number,
  settings: ToolpathSettingsInput = {},
): ToolpathPlanResult {
  return callGeometryRust('mesh_toolpaths', { mesh, zMin, zMax, ...toolpathArguments(settings) })
}
/** Preview dialect after slicer → optimize → emit. */
export function emitPolygonMeshGcode(
  mesh: PolygonMesh,
  zMin: number,
  zMax: number,
  settings: ToolpathSettingsInput = {},
): GcodeExportResult {
  return callGeometryRust('mesh_gcode', { mesh, zMin, zMax, ...toolpathArguments(settings) })
}
/** Machine job + thick `.gcode.3mf` after slicer → optimize → emit_job. */
export function emitPolygonMeshGcodeJob(
  mesh: PolygonMesh,
  zMin: number,
  zMax: number,
  settings: JobSettingsInput = {},
): GcodeJobExportResult {
  return callGeometryRust('mesh_gcode_job', { mesh, zMin, zMax, ...jobArguments(settings) })
}

/** Native dialects parse strictly; other slicers' files use the tolerant reader. */
export function parseGcodePreview(gcode: string): GcodePreviewResult {
  return callGeometryRust('gcode_preview', { gcode })
}
/** Like `parseGcodePreview`, plus detected dialect, generator and firmware flavor. */
export function inspectGcode(gcode: string): GcodeInspectResult {
  return callGeometryRust('gcode_parse', { gcode })
}

export interface PolygonProfile {outer:number[][];holes?:number[][][]}
export const extrudePolygonProfile=(profile:PolygonProfile,vector:number[]):PolygonBuild=>callGeometryRust('polygon_extrude',{profile,vector})
export const revolvePolygonProfile=(profile:number[][],angle=360,segments=32,caps=true):PolygonBuild=>callGeometryRust('polygon_revolve',{profile,angle,segments,caps})
export const loftPolygonSections=(sections:number[][][],caps=true):PolygonBuild=>callGeometryRust('polygon_loft',{sections,caps})
export const sweepPolygonProfile=(profile:number[][],path:number[][],up=[1,0,0],caps=true):PolygonBuild=>callGeometryRust('polygon_sweep',{profile,path,up,caps})

import type {GeometryDeformation,GeometryBrush} from '../geometryEditing'
export const deformPolygonMesh=(mesh:PolygonMesh,deformation:GeometryDeformation):PolygonBuild=>callGeometryRust('polygon_deform',{mesh,deformation})
export const brushPolygonMesh=(mesh:PolygonMesh,brush:GeometryBrush):PolygonBuild=>callGeometryRust('polygon_brush',{mesh,brush})
export const extrudePolygonFaces=(mesh:PolygonMesh,triangles:number[],vector:number[]):PolygonBuild=>callGeometryRust('polygon_extrude_faces',{mesh,triangles,vector})
