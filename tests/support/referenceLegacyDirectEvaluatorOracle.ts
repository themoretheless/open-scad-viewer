import { createHash } from 'node:crypto'

/*
 * Independent differential oracle for the pinned direct parseOpenSCAD evaluator.
 *
 * This file deliberately owns its observable-shape tables, limits, snapshots,
 * scene projection, byte grammar, metric policy, and comparison rules. It must
 * not import the SemanticProgram adapter or any production source module.
 */

const LIMIT = Object.freeze({
  meshes: 1_000,
  triangles: 750_000,
  bytes: 128 * 1024 * 1024,
  warnings: 32,
  warningCharacters: 512,
  errorNameCharacters: 128,
  // OpenSCADParseError includes the complete source line. The direct parser's
  // frozen 250,000-character boundary therefore needs a bounded, nontruncating
  // snapshot for the one-character-over-limit terminal.
  errorMessageCharacters: 250_512,
  identityCharacters: 256,
  sourceLabelCharacters: 512,
  comparisonDiffs: 256,
  canonicalDepth: 64,
})

const KEY = Object.freeze({
  result: ['meshes', 'warnings', 'volume', 'surfaceArea', 'quality', 'reduced', 'timings'],
  timings: ['parseMs', 'bindMs', 'initializeMs', 'evaluateMs', 'analyzeMs'],
  mesh: [
    'entityId', 'geometryAssetId', 'vertices', 'indices', 'bvh', 'edgeIndices',
    'color', 'transform', 'faceIds', 'provenance', 'topology',
  ],
  bvh: ['version', 'vertexStride', 'leafSize', 'nodeCount', 'bounds', 'nodes', 'triangles'],
  provenance: ['triangleStart', 'triangleEnd', 'source', 'backside'],
  source: ['id', 'operationId', 'instanceId', 'originalId', 'start', 'end', 'label'],
  topology: ['boundary', 'crease', 'nonManifold', 'degenerate'],
}) satisfies Readonly<Record<string, readonly string[]>>

export const REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT = Object.freeze({
  manifest: 'legacy-direct-evaluator-v1',
  comparator: 'legacy-direct-differential-v1',
  exact: Object.freeze([
    'outcome tag',
    'error name/code/message and UTF-16 line/column/start/end',
    'warning order and text',
    'quality and reduced',
    'mesh and scene order',
    'vertex/index/BVH/edge/face/transform bytes',
    'topology and provenance runs',
    'dense first-seen provenance original-ID equality partition',
    'colors',
    'entity/operation/geometry-asset identities',
  ]),
  tolerant: Object.freeze({
    volume: Object.freeze({ absolute: 1e-9, relative: 1e-9 }),
    surfaceArea: Object.freeze({ absolute: 1e-9, relative: 1e-9 }),
  }),
  validateOnly: Object.freeze([
    'timings.parseMs/bindMs/initializeMs/evaluateMs/analyzeMs are finite and nonnegative',
    'provenance.source.originalId is a nonnegative process-local handle',
  ]),
  unavailableFromDirectEvaluator: Object.freeze([
    'canonical protocol-v5 scene/wire bytes',
    'multiple diagnostic ordering or warnings retained on failure',
    'queue settlement, backend quiescence, or allocation rollback after cancellation',
    'cross-adapter semantic meaning for phase timing values',
  ]),
  blockingIntegrationRequirements: Object.freeze([
    'candidate path must expose a complete ParseResult-compatible terminal outcome, not only opaque backend payloads',
    'candidate publication must materialize legacy mesh analysis and provenance before byte comparison is possible',
    'protocol-v5 wire/queue/cancellation settlement requires a separate worker-level harness',
  ]),
  workerBoundaryPolicy: Object.freeze({
    identityCharacters: 256,
    oversizedDirectSuccess: 'worker-level-unavailable',
    requiredDisposition: 'bounded-error-before-v5-success-publication',
  }),
})

export type ReferenceLegacyFieldClass =
  | 'outcome'
  | 'error'
  | 'warnings'
  | 'quality'
  | 'preview-policy'
  | 'mesh-bytes'
  | 'scene-bytes'
  | 'color'
  | 'identity'
  | 'provenance'
  | 'metrics'
  | 'schema'

export class ReferenceLegacyOracleError extends TypeError {
  constructor(readonly path: string, detail: string) {
    super(`${path}: ${detail}`)
    this.name = 'ReferenceLegacyOracleError'
  }
}

export interface ReferenceLegacyByteView {
  readonly type: 'Float32Array' | 'Uint32Array'
  readonly length: number
  readonly bytes: Uint8Array
}

export interface ReferenceLegacySourceSnapshot {
  readonly id: number
  readonly operationId: string
  readonly instanceId: string
  /** Dense first-seen canonical ID preserving the process-local equality partition. */
  readonly originalId: number
  readonly start: number
  readonly end: number
  readonly label: string
}

export interface ReferenceLegacyMeshSnapshot {
  readonly entityId: string
  readonly geometryAssetId: string
  readonly vertices: ReferenceLegacyByteView
  readonly indices: ReferenceLegacyByteView
  readonly bvh: Readonly<{
    version: 1
    vertexStride: number
    leafSize: number
    nodeCount: number
    bounds: ReferenceLegacyByteView
    nodes: ReferenceLegacyByteView
    triangles: ReferenceLegacyByteView
  }>
  readonly edgeIndices: ReferenceLegacyByteView
  readonly color: readonly [number, number, number, number]
  readonly transform: ReferenceLegacyByteView
  readonly faceIds: ReferenceLegacyByteView
  readonly provenance: readonly Readonly<{
    triangleStart: number
    triangleEnd: number
    source: ReferenceLegacySourceSnapshot | null
    backside: boolean
  }>[]
  readonly topology: Readonly<{
    boundary: number
    crease: number
    nonManifold: number
    degenerate: number
  }>
}

export interface ReferenceLegacySceneSnapshot {
  readonly version: 1
  readonly projection: 'legacy-mesh-scene-projection-v1'
  readonly assets: readonly Readonly<{
    version: 1
    id: string
    vertices: ReferenceLegacyByteView
    indices: ReferenceLegacyByteView
  }>[]
  readonly entities: readonly Readonly<{
    id: string
    geometryAssetId: string
    color: readonly [number, number, number, number]
    transform: ReferenceLegacyByteView
    inspection: Readonly<{
      state: 'ready'
      version: 1
      bvh: ReferenceLegacyMeshSnapshot['bvh']
      edgeIndices: ReferenceLegacyByteView
      faceIds: ReferenceLegacyByteView
      provenance: ReferenceLegacyMeshSnapshot['provenance']
      topology: ReferenceLegacyMeshSnapshot['topology']
    }>
  }>[]
}

export interface ReferenceLegacySuccessSnapshot {
  readonly warnings: readonly string[]
  readonly volume: number
  readonly surfaceArea: number
  readonly quality: 'preview' | 'full'
  readonly reduced: boolean
  /** Values are intentionally erased after validation. */
  readonly timings: Readonly<{
    parseMs: 0
    bindMs: 0
    initializeMs: 0
    evaluateMs: 0
    analyzeMs: 0
  }>
  readonly meshes: readonly ReferenceLegacyMeshSnapshot[]
  readonly scene: ReferenceLegacySceneSnapshot
}

export type ReferenceLegacyErrorSnapshot = Readonly<{
  name: string
  message: string
  code: string | null
  line: number | null
  column: number | null
  start: number | null
  end: number | null
}>

export type ReferenceLegacyOutcome =
  | Readonly<{ tag: 'success'; success: ReferenceLegacySuccessSnapshot }>
  | Readonly<{ tag: 'error'; error: ReferenceLegacyErrorSnapshot }>
  | Readonly<{ tag: 'cancelled'; error: ReferenceLegacyErrorSnapshot }>

export interface ReferenceLegacyDiff {
  readonly fieldClass: ReferenceLegacyFieldClass
  readonly path: string
  readonly expected: string
  readonly actual: string
}

type Budget = {
  bytes: number
  triangles: number
  originalIds: Map<number, number>
}

function fail(path: string, detail: string): never {
  throw new ReferenceLegacyOracleError(path, detail)
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (value === null || Array.isArray(value) || typeof value !== 'object') {
    fail(path, 'expected a plain record')
  }
  const prototype = Object.getPrototypeOf(value)
  if (prototype !== Object.prototype && prototype !== null) fail(path, 'expected a plain record')
  return value as Record<string, unknown>
}

function exact(value: unknown, keys: readonly string[], path: string): Record<string, unknown> {
  const candidate = record(value, path)
  const descriptors = Object.getOwnPropertyDescriptors(candidate)
  const actual = Reflect.ownKeys(descriptors)
  if (actual.some(key => typeof key !== 'string')
    || actual.length !== keys.length
    || keys.some(key => !Object.hasOwn(descriptors, key))) {
    fail(path, `expected exact keys ${keys.join(', ')}`)
  }
  for (const key of keys) {
    const descriptor = descriptors[key]
    if (!descriptor.enumerable || !Object.hasOwn(descriptor, 'value')) {
      fail(`${path}.${key}`, 'expected an enumerable data property')
    }
  }
  return Object.fromEntries(keys.map(key => [key, descriptors[key].value]))
}

function denseArray(value: unknown, path: string, maximum: number): unknown[] {
  if (!Array.isArray(value) || value.length > maximum) fail(path, `expected a dense array of at most ${maximum} items`)
  const descriptors = Object.getOwnPropertyDescriptors(value)
  const keys = Reflect.ownKeys(descriptors)
  if (keys.some(key => typeof key !== 'string') || keys.length !== value.length + 1) {
    fail(path, 'expected a dense data-property array')
  }
  const lengthDescriptor = descriptors.length
  if (!lengthDescriptor || !Object.hasOwn(lengthDescriptor, 'value') || lengthDescriptor.enumerable) {
    fail(`${path}.length`, 'invalid array length descriptor')
  }
  const output: unknown[] = []
  for (let index = 0; index < value.length; index++) {
    const descriptor = descriptors[String(index)]
    if (!descriptor || !descriptor.enumerable || !Object.hasOwn(descriptor, 'value')) {
      fail(`${path}[${index}]`, 'expected an enumerable data property')
    }
    output.push(descriptor.value)
  }
  return output
}

function finite(value: unknown, path: string, nonnegative = false): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || (nonnegative && value < 0)) {
    fail(path, `expected a ${nonnegative ? 'nonnegative ' : ''}finite number`)
  }
  return Object.is(value, -0) ? 0 : value
}

function integer(value: unknown, path: string, minimum = 0): number {
  if (!Number.isSafeInteger(value) || (value as number) < minimum) {
    fail(path, `expected a safe integer >= ${minimum}`)
  }
  return value as number
}

function booleanValue(value: unknown, path: string): boolean {
  if (typeof value !== 'boolean') fail(path, 'expected a boolean')
  return value
}

function wellFormed(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

function text(value: unknown, path: string, maximum: number): string {
  if (typeof value !== 'string' || value.length > maximum || !wellFormed(value)) {
    fail(path, `expected well-formed text of at most ${maximum} UTF-16 code units`)
  }
  return value
}

function snapshotView(
  value: unknown,
  type: ReferenceLegacyByteView['type'],
  path: string,
  budget: Budget,
): ReferenceLegacyByteView {
  const valid = type === 'Float32Array' ? value instanceof Float32Array : value instanceof Uint32Array
  if (!valid) fail(path, `expected ${type}`)
  const view = value as Float32Array | Uint32Array
  if (type === 'Float32Array' && !view.every(Number.isFinite)) fail(path, 'contains a non-finite float')
  budget.bytes += view.byteLength
  if (budget.bytes > LIMIT.bytes) fail(path, `aggregate typed-array bytes exceed ${LIMIT.bytes}`)
  return Object.freeze({
    type,
    length: view.length,
    bytes: new Uint8Array(view.buffer, view.byteOffset, view.byteLength).slice(),
  })
}

function snapshotSource(
  value: unknown,
  path: string,
  budget: Budget,
): ReferenceLegacySourceSnapshot {
  const source = exact(value, KEY.source, path)
  const operationId = text(source.operationId, `${path}.operationId`, LIMIT.identityCharacters)
  const instanceId = text(source.instanceId, `${path}.instanceId`, LIMIT.identityCharacters)
  if (!operationId.startsWith('op:')) fail(`${path}.operationId`, 'expected op: identity')
  if (!instanceId.startsWith('entity:')) fail(`${path}.instanceId`, 'expected entity: identity')
  const start = integer(source.start, `${path}.start`)
  const end = integer(source.end, `${path}.end`)
  if (end < start) fail(`${path}.end`, 'source span is reversed')
  const processOriginalId = integer(source.originalId, `${path}.originalId`)
  let originalId = budget.originalIds.get(processOriginalId)
  if (originalId === undefined) {
    originalId = budget.originalIds.size
    budget.originalIds.set(processOriginalId, originalId)
  }
  return Object.freeze({
    id: integer(source.id, `${path}.id`),
    operationId,
    instanceId,
    originalId,
    start,
    end,
    label: text(source.label, `${path}.label`, LIMIT.sourceLabelCharacters),
  })
}

function snapshotMesh(value: unknown, index: number, budget: Budget): ReferenceLegacyMeshSnapshot {
  const path = `$.meshes[${index}]`
  const mesh = exact(value, KEY.mesh, path)
  const entityId = text(mesh.entityId, `${path}.entityId`, LIMIT.identityCharacters)
  const geometryAssetId = text(mesh.geometryAssetId, `${path}.geometryAssetId`, LIMIT.identityCharacters)
  if (!entityId.startsWith('entity:')) fail(`${path}.entityId`, 'expected entity: identity')
  if (!geometryAssetId.startsWith('asset:')) fail(`${path}.geometryAssetId`, 'expected asset: identity')

  const vertices = snapshotView(mesh.vertices, 'Float32Array', `${path}.vertices`, budget)
  const indices = snapshotView(mesh.indices, 'Uint32Array', `${path}.indices`, budget)
  if (vertices.length % 6 !== 0) fail(`${path}.vertices`, 'vertex stride must be six floats')
  if (indices.length % 3 !== 0) fail(`${path}.indices`, 'triangle indices must be triples')
  const triangleCount = indices.length / 3
  budget.triangles += triangleCount
  if (budget.triangles > LIMIT.triangles) fail(`${path}.indices`, `aggregate triangles exceed ${LIMIT.triangles}`)

  const bvhValue = exact(mesh.bvh, KEY.bvh, `${path}.bvh`)
  const nodeCount = integer(bvhValue.nodeCount, `${path}.bvh.nodeCount`)
  const bounds = snapshotView(bvhValue.bounds, 'Float32Array', `${path}.bvh.bounds`, budget)
  const nodes = snapshotView(bvhValue.nodes, 'Uint32Array', `${path}.bvh.nodes`, budget)
  const bvhTriangles = snapshotView(bvhValue.triangles, 'Uint32Array', `${path}.bvh.triangles`, budget)
  if (bounds.length !== nodeCount * 6 || nodes.length !== nodeCount * 2
    || bvhTriangles.length > triangleCount) fail(`${path}.bvh`, 'inconsistent compact BVH lengths')
  const bvh = Object.freeze({
    version: integer(bvhValue.version, `${path}.bvh.version`, 1) as 1,
    vertexStride: integer(bvhValue.vertexStride, `${path}.bvh.vertexStride`, 3),
    leafSize: integer(bvhValue.leafSize, `${path}.bvh.leafSize`, 1),
    nodeCount,
    bounds,
    nodes,
    triangles: bvhTriangles,
  })
  if (bvh.version !== 1) fail(`${path}.bvh.version`, 'expected version 1')

  const edgeIndices = snapshotView(mesh.edgeIndices, 'Uint32Array', `${path}.edgeIndices`, budget)
  if (edgeIndices.length % 2 !== 0) fail(`${path}.edgeIndices`, 'edge indices must be pairs')
  const transform = snapshotView(mesh.transform, 'Float32Array', `${path}.transform`, budget)
  if (transform.length !== 16) fail(`${path}.transform`, 'expected a 4x4 transform')
  const faceIds = snapshotView(mesh.faceIds, 'Uint32Array', `${path}.faceIds`, budget)
  if (faceIds.length !== triangleCount) fail(`${path}.faceIds`, 'expected one face ID per triangle')

  const colorValues = denseArray(mesh.color, `${path}.color`, 4)
  if (colorValues.length !== 4) fail(`${path}.color`, 'expected RGBA')
  const color = colorValues.map((item, channel) => {
    const component = finite(item, `${path}.color[${channel}]`)
    if (component < 0 || component > 1) fail(`${path}.color[${channel}]`, 'color must be normalized')
    return component
  }) as [number, number, number, number]

  let previousTriangleEnd = 0
  const provenance = denseArray(mesh.provenance, `${path}.provenance`, triangleCount)
    .map((item, runIndex) => {
      const runPath = `${path}.provenance[${runIndex}]`
      const run = exact(item, KEY.provenance, runPath)
      const triangleStart = integer(run.triangleStart, `${runPath}.triangleStart`)
      const triangleEnd = integer(run.triangleEnd, `${runPath}.triangleEnd`)
      if (triangleEnd <= triangleStart || triangleEnd > triangleCount) {
        fail(runPath, 'invalid triangle interval')
      }
      if (runIndex > 0 && triangleStart < previousTriangleEnd) fail(runPath, 'provenance intervals overlap')
      previousTriangleEnd = triangleEnd
      return Object.freeze({
        triangleStart,
        triangleEnd,
        source: run.source === null ? null : snapshotSource(run.source, `${runPath}.source`, budget),
        backside: booleanValue(run.backside, `${runPath}.backside`),
      })
    })

  const topologyValue = exact(mesh.topology, KEY.topology, `${path}.topology`)
  const topology = Object.freeze({
    boundary: integer(topologyValue.boundary, `${path}.topology.boundary`),
    crease: integer(topologyValue.crease, `${path}.topology.crease`),
    nonManifold: integer(topologyValue.nonManifold, `${path}.topology.nonManifold`),
    degenerate: integer(topologyValue.degenerate, `${path}.topology.degenerate`),
  })

  return Object.freeze({
    entityId,
    geometryAssetId,
    vertices,
    indices,
    bvh,
    edgeIndices,
    color: Object.freeze(color) as readonly [number, number, number, number],
    transform,
    faceIds,
    provenance: Object.freeze(provenance),
    topology,
  })
}

function sameBytes(left: Uint8Array, right: Uint8Array): boolean {
  return left.length === right.length && left.every((byte, index) => byte === right[index])
}

function sameView(left: ReferenceLegacyByteView, right: ReferenceLegacyByteView): boolean {
  return left.type === right.type && left.length === right.length && sameBytes(left.bytes, right.bytes)
}

export function referenceProjectLegacyScene(
  meshes: readonly ReferenceLegacyMeshSnapshot[],
): ReferenceLegacySceneSnapshot {
  const assets: Array<ReferenceLegacySceneSnapshot['assets'][number]> = []
  const assetsById = new Map<string, ReferenceLegacySceneSnapshot['assets'][number]>()
  const entityIds = new Set<string>()
  const entities: Array<ReferenceLegacySceneSnapshot['entities'][number]> = []
  meshes.forEach((mesh, index) => {
    if (entityIds.has(mesh.entityId)) fail(`$.meshes[${index}].entityId`, 'duplicate entity identity')
    entityIds.add(mesh.entityId)
    const existing = assetsById.get(mesh.geometryAssetId)
    if (existing && (!sameView(existing.vertices, mesh.vertices) || !sameView(existing.indices, mesh.indices))) {
      fail(`$.meshes[${index}].geometryAssetId`, 'one asset identity aliases different bytes')
    }
    if (!existing) {
      const asset = Object.freeze({
        version: 1 as const,
        id: mesh.geometryAssetId,
        vertices: mesh.vertices,
        indices: mesh.indices,
      })
      assetsById.set(asset.id, asset)
      assets.push(asset)
    }
    entities.push(Object.freeze({
      id: mesh.entityId,
      geometryAssetId: mesh.geometryAssetId,
      color: mesh.color,
      transform: mesh.transform,
      inspection: Object.freeze({
        state: 'ready' as const,
        version: 1 as const,
        bvh: mesh.bvh,
        edgeIndices: mesh.edgeIndices,
        faceIds: mesh.faceIds,
        provenance: mesh.provenance,
        topology: mesh.topology,
      }),
    }))
  })
  return Object.freeze({
    version: 1,
    projection: 'legacy-mesh-scene-projection-v1',
    assets: Object.freeze(assets),
    entities: Object.freeze(entities),
  })
}

export function referenceSnapshotLegacySuccess(value: unknown): ReferenceLegacySuccessSnapshot {
  const result = exact(value, KEY.result, '$')
  const budget: Budget = { bytes: 0, triangles: 0, originalIds: new Map() }
  const meshes = denseArray(result.meshes, '$.meshes', LIMIT.meshes)
    .map((mesh, index) => snapshotMesh(mesh, index, budget))
  const warnings = denseArray(result.warnings, '$.warnings', LIMIT.warnings)
    .map((warning, index) => text(warning, `$.warnings[${index}]`, LIMIT.warningCharacters))
  const quality = result.quality
  if (quality !== 'preview' && quality !== 'full') fail('$.quality', 'expected preview or full')
  const timingsValue = exact(result.timings, KEY.timings, '$.timings')
  for (const key of KEY.timings) finite(timingsValue[key], `$.timings.${key}`, true)
  return Object.freeze({
    warnings: Object.freeze(warnings),
    volume: finite(result.volume, '$.volume', true),
    surfaceArea: finite(result.surfaceArea, '$.surfaceArea', true),
    quality,
    reduced: booleanValue(result.reduced, '$.reduced'),
    timings: Object.freeze({ parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 }),
    meshes: Object.freeze(meshes),
    scene: referenceProjectLegacyScene(meshes),
  })
}

function inheritedDataProperty(value: object, key: string): unknown {
  let cursor: object | null = value
  for (let depth = 0; cursor !== null && depth < 8; depth++) {
    const descriptor = Object.getOwnPropertyDescriptor(cursor, key)
    if (descriptor) {
      if (!Object.hasOwn(descriptor, 'value')) fail(`$error.${key}`, 'accessor properties are forbidden')
      return descriptor.value
    }
    cursor = Object.getPrototypeOf(cursor)
  }
  return undefined
}

function optionalErrorInteger(value: unknown, path: string, positive = false): number | null {
  if (value === undefined) return null
  return integer(value, path, positive ? 1 : 0)
}

export function referenceSnapshotLegacyError(error: unknown): ReferenceLegacyOutcome {
  if (error === null || (typeof error !== 'object' && typeof error !== 'function')) {
    fail('$error', 'expected an Error-like object')
  }
  const object = error as object
  const name = text(inheritedDataProperty(object, 'name'), '$error.name', LIMIT.errorNameCharacters)
  const message = text(inheritedDataProperty(object, 'message'), '$error.message', LIMIT.errorMessageCharacters)
  const codeValue = inheritedDataProperty(object, 'code')
  const code = codeValue === undefined ? null : text(codeValue, '$error.code', 80)
  const line = optionalErrorInteger(inheritedDataProperty(object, 'line'), '$error.line', true)
  const column = optionalErrorInteger(inheritedDataProperty(object, 'column'), '$error.column', true)
  const start = optionalErrorInteger(inheritedDataProperty(object, 'start'), '$error.start')
  const end = optionalErrorInteger(inheritedDataProperty(object, 'end'), '$error.end')
  if ((start === null) !== (end === null) || (start !== null && end! < start)) {
    fail('$error', 'start/end must be a complete forward span')
  }
  const snapshot = Object.freeze({ name, message, code, line, column, start, end })
  return Object.freeze({ tag: name === 'AbortedError' ? 'cancelled' : 'error', error: snapshot })
}

export async function referenceCaptureLegacyOutcome(
  evaluate: () => Promise<unknown>,
): Promise<ReferenceLegacyOutcome> {
  try {
    return Object.freeze({ tag: 'success', success: referenceSnapshotLegacySuccess(await evaluate()) })
  } catch (error) {
    if (error instanceof ReferenceLegacyOracleError) throw error
    return referenceSnapshotLegacyError(error)
  }
}

function summary(value: unknown): string {
  if (value instanceof Uint8Array) return `bytes(${value.length})`
  if (typeof value === 'string') return JSON.stringify(value.length > 160 ? `${value.slice(0, 157)}...` : value)
  if (Array.isArray(value)) return `array(${value.length})`
  if (value !== null && typeof value === 'object') return `object(${Object.keys(value).join(',')})`
  return String(value)
}

function classify(path: string): ReferenceLegacyFieldClass {
  if (path.startsWith('$.error')) return 'error'
  if (path.startsWith('$.success.warnings')) return 'warnings'
  if (path === '$.success.quality' || path === '$.success.reduced') return 'quality'
  if (path === '$.success.volume' || path === '$.success.surfaceArea') return 'metrics'
  if (path.startsWith('$.success.scene')) return 'scene-bytes'
  if (path.includes('.color')) return 'color'
  if (/\.(?:entityId|geometryAssetId|operationId|instanceId|id)$/.test(path)) return 'identity'
  if (path.includes('.provenance')) return 'provenance'
  if (path.startsWith('$.success.meshes')) return 'mesh-bytes'
  return 'schema'
}

function appendDiff(
  diffs: ReferenceLegacyDiff[],
  fieldClass: ReferenceLegacyFieldClass,
  path: string,
  expected: unknown,
  actual: unknown,
): void {
  if (diffs.length >= LIMIT.comparisonDiffs) return
  diffs.push(Object.freeze({
    fieldClass,
    path,
    expected: summary(expected),
    actual: summary(actual),
  }))
}

function compareExact(
  expected: unknown,
  actual: unknown,
  path: string,
  diffs: ReferenceLegacyDiff[],
): void {
  if (expected instanceof Uint8Array || actual instanceof Uint8Array) {
    if (!(expected instanceof Uint8Array) || !(actual instanceof Uint8Array)) {
      appendDiff(diffs, classify(path), path, expected, actual)
      return
    }
    if (expected.length !== actual.length) {
      appendDiff(diffs, classify(path), `${path}.length`, expected.length, actual.length)
      return
    }
    for (let index = 0; index < expected.length; index++) {
      if (expected[index] !== actual[index]) {
        appendDiff(diffs, classify(path), `${path}[${index}]`, expected[index], actual[index])
        return
      }
    }
    return
  }
  if (Array.isArray(expected) || Array.isArray(actual)) {
    if (!Array.isArray(expected) || !Array.isArray(actual)) {
      appendDiff(diffs, classify(path), path, expected, actual)
      return
    }
    if (expected.length !== actual.length) {
      appendDiff(diffs, classify(path), `${path}.length`, expected.length, actual.length)
    }
    const length = Math.min(expected.length, actual.length)
    for (let index = 0; index < length; index++) compareExact(expected[index], actual[index], `${path}[${index}]`, diffs)
    return
  }
  if (expected !== null && actual !== null && typeof expected === 'object' && typeof actual === 'object') {
    const expectedRecord = expected as Record<string, unknown>
    const actualRecord = actual as Record<string, unknown>
    const expectedKeys = Object.keys(expectedRecord).sort()
    const actualKeys = Object.keys(actualRecord).sort()
    if (expectedKeys.join('\0') !== actualKeys.join('\0')) {
      appendDiff(diffs, classify(path), `${path}.$keys`, expectedKeys, actualKeys)
    }
    for (const key of expectedKeys) {
      if (Object.hasOwn(actualRecord, key)) compareExact(expectedRecord[key], actualRecord[key], `${path}.${key}`, diffs)
    }
    return
  }
  if (!Object.is(expected, actual)) appendDiff(diffs, classify(path), path, expected, actual)
}

function metricEqual(expected: number, actual: number, absolute: number, relative: number): boolean {
  const difference = Math.abs(expected - actual)
  return difference <= absolute + relative * Math.max(Math.abs(expected), Math.abs(actual))
}

function compareSuccess(
  expected: ReferenceLegacySuccessSnapshot,
  actual: ReferenceLegacySuccessSnapshot,
  diffs: ReferenceLegacyDiff[],
  ignoreQuality = false,
): void {
  compareExact(expected.warnings, actual.warnings, '$.success.warnings', diffs)
  if (!ignoreQuality) {
    compareExact(expected.quality, actual.quality, '$.success.quality', diffs)
    compareExact(expected.reduced, actual.reduced, '$.success.reduced', diffs)
  }
  const volumePolicy = REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT.tolerant.volume
  if (!metricEqual(expected.volume, actual.volume, volumePolicy.absolute, volumePolicy.relative)) {
    appendDiff(diffs, 'metrics', '$.success.volume', expected.volume, actual.volume)
  }
  const areaPolicy = REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT.tolerant.surfaceArea
  if (!metricEqual(expected.surfaceArea, actual.surfaceArea, areaPolicy.absolute, areaPolicy.relative)) {
    appendDiff(diffs, 'metrics', '$.success.surfaceArea', expected.surfaceArea, actual.surfaceArea)
  }
  compareExact(expected.meshes, actual.meshes, '$.success.meshes', diffs)
  compareExact(expected.scene, actual.scene, '$.success.scene', diffs)
}

export function referenceCompareLegacyOutcomes(
  expected: ReferenceLegacyOutcome,
  actual: ReferenceLegacyOutcome,
): readonly ReferenceLegacyDiff[] {
  const diffs: ReferenceLegacyDiff[] = []
  if (expected.tag !== actual.tag) {
    appendDiff(diffs, 'outcome', '$.tag', expected.tag, actual.tag)
    return Object.freeze(diffs)
  }
  if (expected.tag === 'success' && actual.tag === 'success') {
    compareSuccess(expected.success, actual.success, diffs)
  } else if (expected.tag !== 'success' && actual.tag !== 'success') {
    compareExact(expected.error, actual.error, '$.error', diffs)
  }
  return Object.freeze(diffs)
}

function stableReducedMeshProjection(mesh: ReferenceLegacyMeshSnapshot): unknown {
  const sources: ReferenceLegacySourceSnapshot[] = []
  const seen = new Set<string>()
  for (const run of mesh.provenance) {
    if (run.source === null) continue
    const key = JSON.stringify(run.source)
    if (!seen.has(key)) {
      seen.add(key)
      sources.push(run.source)
    }
  }
  return {
    entityId: mesh.entityId,
    color: mesh.color,
    transform: mesh.transform,
    sources,
  }
}

export function referenceCompareLegacyPreviewFull(
  preview: ReferenceLegacyOutcome,
  full: ReferenceLegacyOutcome,
): readonly ReferenceLegacyDiff[] {
  const diffs: ReferenceLegacyDiff[] = []
  if (preview.tag !== 'success' || full.tag !== 'success') {
    appendDiff(diffs, 'outcome', '$.previewFull.tag', 'success/success', `${preview.tag}/${full.tag}`)
    return Object.freeze(diffs)
  }
  if (preview.success.quality !== 'preview') {
    appendDiff(diffs, 'preview-policy', '$.preview.quality', 'preview', preview.success.quality)
  }
  if (full.success.quality !== 'full') {
    appendDiff(diffs, 'preview-policy', '$.full.quality', 'full', full.success.quality)
  }
  if (full.success.reduced) appendDiff(diffs, 'preview-policy', '$.full.reduced', false, true)

  if (!preview.success.reduced) {
    compareSuccess(preview.success, full.success, diffs, true)
    return Object.freeze(diffs)
  }
  const previewProjection = preview.success.meshes.map(stableReducedMeshProjection)
  const fullProjection = full.success.meshes.map(stableReducedMeshProjection)
  compareExact(previewProjection, fullProjection, '$.previewFull.stableMeshes', diffs)
  return Object.freeze(diffs)
}

class Writer {
  private readonly chunks: Uint8Array[] = []
  private length = 0

  push(bytes: Uint8Array): void {
    this.length += bytes.length
    if (this.length > LIMIT.bytes) fail('$bytes', `canonical bytes exceed ${LIMIT.bytes}`)
    this.chunks.push(bytes)
  }

  byte(value: number): void { this.push(Uint8Array.of(value)) }

  u32(value: number): void {
    const bytes = new Uint8Array(4)
    new DataView(bytes.buffer).setUint32(0, value, false)
    this.push(bytes)
  }

  f64(value: number): void {
    const bytes = new Uint8Array(8)
    new DataView(bytes.buffer).setFloat64(0, Object.is(value, -0) ? 0 : value, false)
    this.push(bytes)
  }

  finish(): Uint8Array {
    const output = new Uint8Array(this.length)
    let offset = 0
    for (const chunk of this.chunks) {
      output.set(chunk, offset)
      offset += chunk.length
    }
    return output
  }
}

const utf8 = new TextEncoder()

function encodeValue(value: unknown, writer: Writer, depth = 0): void {
  if (depth > LIMIT.canonicalDepth) fail('$bytes', 'canonical value exceeds depth limit')
  if (value === null) { writer.byte(0); return }
  if (value === false) { writer.byte(1); return }
  if (value === true) { writer.byte(2); return }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) fail('$bytes', 'canonical number must be finite')
    writer.byte(3)
    writer.f64(value)
    return
  }
  if (typeof value === 'string') {
    const bytes = utf8.encode(value)
    writer.byte(4)
    writer.u32(bytes.length)
    writer.push(bytes)
    return
  }
  if (value instanceof Uint8Array) {
    writer.byte(5)
    writer.u32(value.length)
    writer.push(value)
    return
  }
  if (Array.isArray(value)) {
    writer.byte(6)
    writer.u32(value.length)
    value.forEach(item => encodeValue(item, writer, depth + 1))
    return
  }
  if (typeof value === 'object') {
    const candidate = value as Record<string, unknown>
    const keys = Object.keys(candidate).sort()
    writer.byte(7)
    writer.u32(keys.length)
    for (const key of keys) {
      encodeValue(key, writer, depth + 1)
      encodeValue(candidate[key], writer, depth + 1)
    }
    return
  }
  fail('$bytes', `unsupported canonical value ${typeof value}`)
}

function encodeFrame(magic: string, value: unknown): Uint8Array {
  const writer = new Writer()
  writer.push(utf8.encode(magic))
  encodeValue(value, writer)
  return writer.finish()
}

function requireSuccess(outcome: ReferenceLegacyOutcome): ReferenceLegacySuccessSnapshot {
  if (outcome.tag !== 'success') fail('$outcome', 'expected a successful outcome')
  return outcome.success
}

/** Independent comparable mesh artifact, not a production protocol frame. */
export function referenceLegacyMeshBytes(outcome: ReferenceLegacyOutcome): Uint8Array {
  return encodeFrame('LME1', requireSuccess(outcome).meshes)
}

/** Independent deterministic scene projection, not protocol-v5 scene/wire bytes. */
export function referenceLegacySceneBytes(outcome: ReferenceLegacyOutcome): Uint8Array {
  return encodeFrame('LSE1', requireSuccess(outcome).scene)
}

export function referenceLegacySha256(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex')
}
