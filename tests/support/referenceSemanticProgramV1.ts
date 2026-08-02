import { createHash } from 'node:crypto'

/*
 * Independent qualification oracle for semantic-program-contract-v1.
 *
 * This module deliberately owns every wire tag, exact-key table, capability
 * mapping, identity preimage and SPE1/SPC1 encoding rule that it uses. Keep it
 * free of production imports: shared constants would make differential tests
 * incapable of detecting coordinated drift in the implementation under test.
 */

export type ReferenceSemanticProgramV1 = Record<string, any>

export type ReferenceSemanticFailureFamily =
  | 'schema'
  | 'field'
  | 'version'
  | 'limit'
  | 'number'
  | 'reference'
  | 'type'
  | 'graph'
  | 'identity'
  | 'source'
  | 'closure'
  | 'order'
  | 'binary'

export class ReferenceSemanticProgramError extends TypeError {
  constructor(
    readonly family: ReferenceSemanticFailureFamily,
    readonly path: string,
    detail: string,
  ) {
    super(`${family} at ${path}: ${detail}`)
    this.name = 'ReferenceSemanticProgramError'
  }
}

const LIMIT = Object.freeze({
  binaryBytes: 64 * 1024 * 1024,
  snapshotValues: 1_000_000,
  snapshotDepth: 256,
  identityDepth: 32,
  stringCodeUnits: 250_000,
  sourceUtf8Bytes: 4_000_000,
  sourceUtf16Units: 250_000,
  nodes: 25_000,
  operations: 50_000,
  occurrences: 100_000,
  outputs: 1_000,
  diagnostics: 10_000,
  // Every effect owns at least one retained node, so the DAG limit is the
  // closed semantic bound. In particular, 1,001 eager empty-base differences
  // remain representable rather than becoming an arbitrary policy failure.
  discardedEffects: 25_000,
  tessellationIntents: 50_000,
  declaredCapabilities: 32,
  capabilityClosure: 128,
})

const KEY = Object.freeze({
  envelope: ['schema', 'schemaVersion', 'source', 'core', 'provenance', 'tessellationIntents', 'diagnostics'],
  version: ['major', 'minor'],
  source: ['sha256', 'utf8ByteLength', 'utf16CodeUnitLength'],
  core: [
    'schema', 'schemaVersion', 'requiredFeatures', 'identityVersion', 'language',
    'units', 'operations', 'occurrences', 'nodes', 'execution', 'result', 'declaredCapabilities',
    'capabilityClosure', 'diagnosticTemplates',
  ],
  language: ['contract', 'semanticsRevision', 'capabilityGraphVersion'],
  units: ['length', 'angle', 'handedness', 'upAxis', 'matrixLayout', 'composition'],
  operation: [
    'id', 'operationId', 'parent', 'childOrdinal', 'name', 'category',
    'structuralPath', 'identityEvidence', 'ambiguityGroup',
  ],
  pathSegment: ['kind', 'name', 'ordinal'],
  occurrence: [
    'id', 'occurrenceId', 'operation', 'parent', 'staticParent', 'dynamicSlots', 'node',
    'outputOrdinal', 'sceneEntityId',
  ],
  dynamicSlot: ['name', 'value', 'duplicateOrdinal'],
  valueType: ['geometryKind', 'space', 'representation', 'evidence'],
  output: ['node', 'producerOccurrence', 'identityOccurrence', 'color'],
  execution: ['version', 'evaluationOrder', 'discardedEffects', 'terminal'],
  discardedEffect: ['tag', 'root', 'ownerOccurrence'],
  terminal: ['tag', 'occurrence', 'diagnosticTemplate', 'prefixFrontier'],
  terminalPrefix: ['root', 'ownerOccurrence'],
  diagnosticTemplate: ['id', 'code', 'severity', 'operation', 'arguments'],
  diagnosticArgument: ['name', 'value'],
  provenance: ['operation', 'span', 'label'],
  span: ['start', 'end'],
  tessellation: ['occurrence', 'chordTolerance', 'angularToleranceDegrees', 'minSegments', 'maxSegments'],
  diagnostic: ['template', 'message', 'span'],
}) satisfies Readonly<Record<string, readonly string[]>>

const NODE_KEY = Object.freeze({
  box: ['id', 'kind', 'valueType', 'size', 'center'],
  'sphere-analytic': ['id', 'kind', 'valueType', 'radius'],
  'sphere-polygonal': ['id', 'kind', 'valueType', 'radius', 'radialSegments'],
  'cylinder-analytic': ['id', 'kind', 'valueType', 'height', 'radiusBottom', 'radiusTop', 'center'],
  'cylinder-polygonal': ['id', 'kind', 'valueType', 'height', 'radiusBottom', 'radiusTop', 'center', 'radialSegments'],
  polyhedron: ['id', 'kind', 'valueType', 'vertices', 'triangles'],
  rectangle: ['id', 'kind', 'valueType', 'size', 'center'],
  'circle-analytic': ['id', 'kind', 'valueType', 'radius'],
  'circle-polygonal': ['id', 'kind', 'valueType', 'radius', 'radialSegments'],
  polygon: ['id', 'kind', 'valueType', 'rings', 'fillRule'],
  transform: ['id', 'kind', 'valueType', 'input', 'matrix'],
  boolean: ['id', 'kind', 'valueType', 'operation', 'inputs'],
  hull: ['id', 'kind', 'valueType', 'inputs'],
  'linear-extrude': ['id', 'kind', 'valueType', 'input', 'height', 'twistDegrees', 'slices', 'scale', 'center'],
  'rotate-extrude-analytic': ['id', 'kind', 'valueType', 'input', 'angleDegrees'],
  'rotate-extrude-polygonal': ['id', 'kind', 'valueType', 'input', 'angleDegrees', 'radialSegments'],
  projection: ['id', 'kind', 'valueType', 'input', 'cut'],
  offset: ['id', 'kind', 'valueType', 'input', 'distance'],
}) satisfies Readonly<Record<string, readonly string[]>>

const EVIDENCE_KEY = Object.freeze({
  'representation-preserving': ['tag'],
  'certified-approximation': ['tag', 'certificateProfile', 'certificatePolicyHash'],
}) satisfies Readonly<Record<string, readonly string[]>>

const IDENTITY_KEY = Object.freeze({
  undefined: ['tag'],
  null: ['tag'],
  boolean: ['tag', 'value'],
  number: ['tag', 'value'],
  string: ['tag', 'value'],
  vector: ['tag', 'items'],
}) satisfies Readonly<Record<string, readonly string[]>>

const TAG = Object.freeze({
  language: ['legacy/current', 'openscad-viewer/brep-1'],
  operationCategory: ['geometry', 'transform', 'boolean', 'control', 'module', 'assertion', 'presentation'],
  pathKind: ['call', 'module', 'control', 'branch', 'body'],
  identity: ['undefined', 'null', 'boolean', 'number', 'string', 'vector'],
  result: ['empty', 'single', 'multi'],
  geometryKind: ['curve', 'wire', 'region', 'sheet', 'solid', 'solid-set'],
  space: ['d2', 'd3'],
  representation: ['analytic-brep', 'rational-brep', 'certified-approx-brep', 'mesh'],
  evidence: ['representation-preserving', 'certified-approximation'],
}) satisfies Readonly<Record<string, readonly string[]>>

const CAPABILITY_BY_KIND = Object.freeze({
  box: 'construct.box',
  'sphere-analytic': 'construct.sphere.analytic',
  'sphere-polygonal': 'construct.sphere.polygonal',
  'cylinder-analytic': 'construct.cylinder.analytic',
  'cylinder-polygonal': 'construct.cylinder.polygonal',
  polyhedron: 'construct.polyhedron',
  rectangle: 'construct.rectangle',
  'circle-analytic': 'construct.circle.analytic',
  'circle-polygonal': 'construct.circle.polygonal',
  polygon: 'construct.polygon',
  transform: 'operation.transform',
  boolean: 'operation.boolean',
  hull: 'operation.hull',
  'linear-extrude': 'operation.linear-extrude',
  'rotate-extrude-analytic': 'operation.rotate-extrude.analytic',
  'rotate-extrude-polygonal': 'operation.rotate-extrude.polygonal',
  projection: 'operation.projection',
  offset: 'operation.offset',
}) satisfies Readonly<Record<string, string>>

const WIRE_TAG = Object.freeze({
  null: 0,
  false: 1,
  true: 2,
  number: 3,
  string: 4,
  array: 5,
  object: 6,
})

const BINARY_STRING_BYTES = 16 * 1024 * 1024

const UTF8 = new TextEncoder()
const IDENTIFIER = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/

function fail(
  family: ReferenceSemanticFailureFamily,
  path: string,
  detail: string,
): never {
  throw new ReferenceSemanticProgramError(family, path, detail)
}

function isRecord(value: unknown): value is Record<string, any> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function record(value: unknown, path: string): Record<string, any> {
  if (!isRecord(value)) fail('schema', path, 'expected object')
  return value
}

function exact(value: unknown, keys: readonly string[], path: string): Record<string, any> {
  const output = record(value, path)
  const actual = Object.keys(output).sort()
  const wanted = [...keys].sort()
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail('field', path, 'unknown or missing fields')
  }
  return output
}

function array(value: unknown, path: string, maximum: number): any[] {
  if (!Array.isArray(value)) fail('schema', path, 'expected array')
  if (value.length > maximum) fail('limit', path, `array exceeds ${maximum} items`)
  return value
}

function wellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

function text(value: unknown, path: string, maximum = 512): string {
  if (typeof value !== 'string' || !wellFormedUnicode(value)) {
    fail('field', path, 'expected well-formed Unicode string')
  }
  if (value.length > maximum) fail('limit', path, `string exceeds ${maximum} UTF-16 units`)
  return value
}

function number(value: unknown, path: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || Object.is(value, -0)) {
    fail('number', path, 'expected finite canonical binary64')
  }
  return value
}

function integer(
  value: unknown,
  path: string,
  minimum = 0,
  maximum = Number.MAX_SAFE_INTEGER,
): number {
  const output = number(value, path)
  if (!Number.isSafeInteger(output) || output < minimum || output > maximum) {
    fail('number', path, `expected integer in [${minimum}, ${maximum}]`)
  }
  return output
}

function positive(value: unknown, path: string, zeroAllowed = false): number {
  const output = number(value, path)
  if (zeroAllowed ? output < 0 : output <= 0) fail('number', path, 'expected positive magnitude')
  return output
}

function boolean(value: unknown, path: string): boolean {
  if (typeof value !== 'boolean') fail('field', path, 'expected boolean')
  return value
}

function tuple(value: unknown, length: number, path: string): number[] {
  const output = array(value, path, length)
  if (output.length !== length) fail('field', path, `expected ${length} components`)
  return output.map((item, index) => number(item, `${path}[${index}]`))
}

function includes(table: readonly string[], value: unknown): value is string {
  return typeof value === 'string' && table.includes(value)
}

function byteCompare(left: Uint8Array, right: Uint8Array): number {
  const shared = Math.min(left.length, right.length)
  for (let index = 0; index < shared; index++) {
    if (left[index] !== right[index]) return left[index] - right[index]
  }
  return left.length - right.length
}

function utf8Compare(left: string, right: string): number {
  return byteCompare(UTF8.encode(left), UTF8.encode(right))
}

function snapshotJsonTree(input: unknown): unknown {
  const ancestors = new Set<object>()
  let values = 0
  let stringBytes = 0
  const addString = (value: string, path: string): void => {
    stringBytes += UTF8.encode(value).length
    if (stringBytes > BINARY_STRING_BYTES) fail('limit', path, 'canonical string/key bytes exceed 16 MiB')
  }
  const visit = (value: unknown, path: string, depth: number): unknown => {
    if (++values > LIMIT.snapshotValues) fail('limit', path, 'snapshot value budget exceeded')
    if (depth > LIMIT.snapshotDepth) fail('limit', path, 'snapshot depth exceeded')
    if (value === null || typeof value === 'boolean') return value
    if (typeof value === 'number') return number(value, path)
    if (typeof value === 'string') {
      text(value, path, LIMIT.stringCodeUnits)
      addString(value, path)
      return value
    }
    if (typeof value !== 'object') fail('schema', path, 'only JSON values are allowed')
    if (Object.getOwnPropertySymbols(value).length !== 0) fail('field', path, 'symbol fields are forbidden')
    if (ancestors.has(value)) fail('graph', path, 'wire object graph cannot be cyclic')
    ancestors.add(value)
    try {
      if (Array.isArray(value)) {
        const keys = Reflect.ownKeys(value)
        if (keys.length !== value.length + 1 || keys[keys.length - 1] !== 'length'
          || keys.slice(0, -1).some((key, index) => key !== String(index))) {
          fail('field', path, 'arrays must be dense without hidden or extra fields')
        }
        const output: unknown[] = []
        for (let index = 0; index < value.length; index++) {
          const descriptor = Object.getOwnPropertyDescriptor(value, String(index))
          if (descriptor === undefined || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) {
            fail('field', `${path}[${index}]`, 'array entries must be enumerable data properties')
          }
          output.push(visit(descriptor.value, `${path}[${index}]`, depth + 1))
        }
        return output
      } else {
        const prototype = Object.getPrototypeOf(value)
        if (prototype !== Object.prototype && prototype !== null) {
          fail('schema', path, 'only plain JSON objects are allowed')
        }
        const output: Record<string, unknown> = Object.create(null)
        for (const key of Reflect.ownKeys(value)) {
          if (typeof key !== 'string') fail('field', path, 'symbol fields are forbidden')
          if (!wellFormedUnicode(key)) fail('field', path, 'object key is not well-formed Unicode')
          addString(key, `${path}.<key>`)
          const descriptor = Object.getOwnPropertyDescriptor(value, key)
          if (descriptor === undefined || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) {
            fail('field', `${path}.${key}`, 'object fields must be enumerable data properties')
          }
          Object.defineProperty(output, key, {
            value: visit(descriptor.value, `${path}.${key}`, depth + 1),
            enumerable: true,
            configurable: true,
            writable: true,
          })
        }
        return output
      }
    } finally {
      ancestors.delete(value)
    }
  }
  return visit(input, '$', 0)
}

function sha256(value: Uint8Array | string): string {
  return createHash('sha256').update(value).digest('hex')
}

function u32be(value: number): Uint8Array {
  const output = new Uint8Array(4)
  new DataView(output.buffer).setUint32(0, value, false)
  return output
}

function lengthPrefixed(domain: Uint8Array, payload: Uint8Array): Uint8Array {
  const output = new Uint8Array(8 + domain.length + payload.length)
  output.set(u32be(domain.length), 0)
  output.set(domain, 4)
  output.set(u32be(payload.length), 4 + domain.length)
  output.set(payload, 8 + domain.length)
  return output
}

function stableJson(value: unknown): string {
  if (value === null || typeof value === 'boolean' || typeof value === 'number' || typeof value === 'string') {
    return JSON.stringify(value)
  }
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`
  return `{${Object.entries(value as Record<string, unknown>)
    .sort(([left], [right]) => utf8Compare(left, right))
    .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`)
    .join(',')}}`
}

function identityDigest(domain: string, payload: unknown): string {
  const domainBytes = UTF8.encode(domain)
  const payloadBytes = UTF8.encode(stableJson(payload))
  return sha256(lengthPrefixed(domainBytes, payloadBytes))
}

export function referenceSemanticOperationIdV1(path: readonly unknown[]): string {
  return `opv1:${identityDigest('semantic-operation-v1', path)}`
}

export function referenceSemanticAmbiguityGroupIdV1(
  parentPath: readonly unknown[],
  category: string,
  name: string,
): string {
  return `ambv1:${identityDigest('semantic-ambiguity-group-v1', { parentPath, category, name })}`
}

export function referenceSemanticOccurrenceIdV1(
  parentOccurrenceId: string | null,
  staticParentOccurrenceId: string | null,
  operationId: string,
  dynamicSlots: readonly unknown[],
): string {
  return `occv1:${identityDigest('semantic-occurrence-v1', {
    parentOccurrenceId,
    staticParentOccurrenceId,
    operationId,
    dynamicSlots,
  })}`
}

export function referenceSemanticSceneEntityIdV1(
  occurrenceId: string,
  outputOrdinal: number,
): string {
  return `entity:v2:${identityDigest('semantic-scene-entity-v1', { occurrenceId, outputOrdinal })}`
}

export function referenceSemanticSourceDescriptorV1(source: string): Record<string, string | number> {
  if (!wellFormedUnicode(source)) fail('binary', '$source', 'source is not well-formed Unicode')
  if (source.length > LIMIT.sourceUtf16Units) {
    fail('limit', '$source', 'source exceeds the UTF-16 code-unit limit')
  }
  const bytes = UTF8.encode(source)
  if (bytes.length > LIMIT.sourceUtf8Bytes) fail('limit', '$source', 'source exceeds the UTF-8 byte limit')
  return {
    sha256: sha256(bytes),
    utf8ByteLength: bytes.length,
    utf16CodeUnitLength: source.length,
  }
}

function visibleRoutingLine(
  raw: string,
  blockAtStart: boolean,
  stringAtStart: boolean,
  escapedAtStart: boolean,
): { line: string; block: boolean; string: boolean; escaped: boolean } {
  let line = stringAtStart ? '"' : ''
  let block = blockAtStart
  let string = stringAtStart
  let escaped = escapedAtStart
  for (let index = 0; index < raw.length; index++) {
    const current = raw[index]
    const next = raw[index + 1]
    if (block) {
      if (current === '*' && next === '/') { block = false; index++ }
      continue
    }
    if (string) {
      if (escaped) escaped = false
      else if (current === '\\') escaped = true
      else if (current === '"') { string = false; line += '"' }
      continue
    }
    if (current === '"') { string = true; line += current; continue }
    if (current === '/' && next === '*') { block = true; index++; continue }
    if (current === '/' && next === '/') { line += raw.slice(index); break }
    line += current
  }
  return { line, block, string, escaped }
}

function referenceRoutingHeader(source: string): { contract: string; capabilities: string[] } {
  let contract = 'legacy/current'
  let languageLine: number | null = null
  let bodyStarted = false
  let block = false
  let string = false
  let escaped = false
  const capabilities = new Set<string>()
  for (const [index, raw] of source.split(/\r\n|\n|\r/).entries()) {
    const visible = visibleRoutingLine(raw, block, string, escaped)
    block = visible.block
    string = visible.string
    escaped = visible.escaped
    const comment = visible.line.indexOf('//')
    const directive = comment >= 0 ? visible.line.slice(comment) : visible.line
    const codeBefore = comment > 0 && visible.line.slice(0, comment).trim() !== ''
    const language = /^\s*\/\/\s*@language\s+(\S+)\s*$/.exec(directive)
    const requires = /^\s*\/\/\s*@requires\s+(.+?)\s*$/.exec(directive)
    const engine = /^\s*\/\/\s*@engine(?:\s+.*)?$/.exec(directive)
    if (language) {
      if (bodyStarted || codeBefore || languageLine !== null
        || !TAG.language.includes(language[1])) {
        fail('source', '$source', `invalid @language directive on line ${index + 1}`)
      }
      contract = language[1]
      languageLine = index + 1
      continue
    }
    if (requires) {
      const identifiers = requires[1].split(/[\s,]+/).filter(Boolean)
      if (bodyStarted || codeBefore || identifiers.length === 0
        || identifiers.some(identifier => !IDENTIFIER.test(identifier))) {
        fail('source', '$source', `invalid @requires directive on line ${index + 1}`)
      }
      identifiers.forEach(identifier => capabilities.add(identifier))
      if (capabilities.size > LIMIT.declaredCapabilities) fail('limit', '$source', 'too many source requirements')
      continue
    }
    if (engine || /^\s*\/\/\s*@(language|requires|engine)\b/i.test(directive)) {
      fail('source', '$source', `malformed reserved routing directive on line ${index + 1}`)
    }
    if (visible.line.trim() !== '' && !/^\s*\/\//.test(visible.line)) bodyStarted = true
  }
  return { contract, capabilities: [...capabilities].sort() }
}

function sortedIdentifiers(
  value: unknown,
  path: string,
  maximum: number,
): string[] {
  const output = array(value, path, maximum).map((item, index) => {
    const identifier = text(item, `${path}[${index}]`, 128)
    if (!IDENTIFIER.test(identifier)) fail('field', `${path}[${index}]`, 'invalid identifier')
    return identifier
  })
  const canonical = [...new Set(output)].sort(utf8Compare)
  if (canonical.length !== output.length || canonical.some((item, index) => item !== output[index])) {
    fail('order', path, 'identifiers must be unique and raw-UTF8 sorted')
  }
  return output
}

function validateIdentityValue(value: unknown, path: string, depth = 0): void {
  if (depth > LIMIT.identityDepth) fail('limit', path, 'identity nesting exceeds limit')
  const identity = record(value, path)
  if (typeof identity.tag !== 'string') fail('schema', path, 'expected typed identity discriminant')
  if (!includes(TAG.identity, identity.tag)) fail('field', `${path}.tag`, 'unknown identity tag')
  exact(identity, (IDENTITY_KEY as Readonly<Record<string, readonly string[]>>)[identity.tag], path)
  if (identity.tag === 'boolean') boolean(identity.value, `${path}.value`)
  if (identity.tag === 'number') number(identity.value, `${path}.value`)
  if (identity.tag === 'string') text(identity.value, `${path}.value`, 4_096)
  if (identity.tag === 'vector') {
    array(identity.items, `${path}.items`, 100_000)
      .forEach((item, index) => validateIdentityValue(item, `${path}.items[${index}]`, depth + 1))
  }
}

function samePath(left: readonly any[], right: readonly any[]): boolean {
  return left.length === right.length && left.every((segment, index) => (
    segment.kind === right[index].kind
    && segment.name === right[index].name
    && segment.ordinal === right[index].ordinal
  ))
}

function validateOperations(value: unknown): Record<string, any>[] {
  const operations = array(value, '$.core.operations', LIMIT.operations)
    .map((item, index) => record(item, `$.core.operations[${index}]`))
  const siblingCounts = new Map<string, number>()
  const siblingOrdinals = new Map<string, number[]>()
  const ambiguityCounts = new Map<string, number>()
  const operationIds = new Set<string>()
  const structuralPaths = new Set<string>()
  const preorderStack: number[] = []
  const closedOperations = new Set<number>()

  operations.forEach((operation, index) => {
    const path = `$.core.operations[${index}]`
    exact(operation, KEY.operation, path)
    if (operation.id !== index) fail('order', `${path}.id`, 'ID must equal array position')
    const parent = operation.parent === null
      ? null
      : integer(operation.parent, `${path}.parent`, 0, index - 1)
    while (preorderStack.length > 0 && preorderStack.at(-1) !== parent) {
      closedOperations.add(preorderStack.pop()!)
    }
    if (parent !== null && (closedOperations.has(parent) || preorderStack.at(-1) !== parent)) {
      fail('order', `${path}.parent`, 'operations are not in parent-before-child preorder')
    }
    const childOrdinal = integer(operation.childOrdinal, `${path}.childOrdinal`, 0, 1_000_000)
    const name = text(operation.name, `${path}.name`, 256)
    if (!includes(TAG.operationCategory, operation.category)) {
      fail('field', `${path}.category`, 'unknown operation category')
    }
    const structuralPath = array(operation.structuralPath, `${path}.structuralPath`, 512)
    if (structuralPath.length === 0) fail('identity', `${path}.structuralPath`, 'path cannot be empty')
    structuralPath.forEach((item, segmentIndex) => {
      const segmentPath = `${path}.structuralPath[${segmentIndex}]`
      const segment = exact(item, KEY.pathSegment, segmentPath)
      if (!includes(TAG.pathKind, segment.kind)) fail('field', `${segmentPath}.kind`, 'unknown path tag')
      text(segment.name, `${segmentPath}.name`, 256)
      integer(segment.ordinal, `${segmentPath}.ordinal`, 0, 1_000_000)
    })
    const last = structuralPath.at(-1)
    if (last.name !== name || last.ordinal !== childOrdinal) {
      fail('identity', path, 'operation fields disagree with final structural path segment')
    }
    if (parent === null) {
      if (structuralPath.length !== 1) fail('identity', `${path}.structuralPath`, 'root path must have one segment')
    } else {
      const parentPath = operations[parent].structuralPath
      if (structuralPath.length !== parentPath.length + 1 || !samePath(structuralPath.slice(0, -1), parentPath)) {
        fail('identity', `${path}.structuralPath`, 'child path must extend parent exactly once')
      }
    }
    const expectedId = referenceSemanticOperationIdV1(structuralPath)
    if (operation.operationId !== expectedId) fail('identity', `${path}.operationId`, 'digest mismatch')
    const pathKey = stableJson(structuralPath)
    if (operationIds.has(expectedId) || structuralPaths.has(pathKey)) {
      fail('identity', `${path}.operationId`, 'static operation identity collision')
    }
    operationIds.add(expectedId)
    structuralPaths.add(pathKey)

    const siblingKey = JSON.stringify([parent, operation.category, name])
    siblingCounts.set(siblingKey, (siblingCounts.get(siblingKey) ?? 0) + 1)
    const ordinals = siblingOrdinals.get(siblingKey) ?? []
    ordinals.push(childOrdinal)
    siblingOrdinals.set(siblingKey, ordinals)
    if (operation.identityEvidence === 'structural-unique') {
      if (operation.ambiguityGroup !== null) fail('identity', `${path}.ambiguityGroup`, 'unique operation has ambiguity group')
    } else if (operation.identityEvidence === 'same-name-positional') {
      const parentPath = parent === null ? [] : operations[parent].structuralPath
      const group = referenceSemanticAmbiguityGroupIdV1(parentPath, operation.category, name)
      if (operation.ambiguityGroup !== group) fail('identity', `${path}.ambiguityGroup`, 'ambiguity digest mismatch')
      ambiguityCounts.set(group, (ambiguityCounts.get(group) ?? 0) + 1)
    } else {
      fail('field', `${path}.identityEvidence`, 'unknown evidence tag')
    }
    preorderStack.push(index)
  })

  operations.forEach((operation, index) => {
    const key = JSON.stringify([operation.parent, operation.category, operation.name])
    const ambiguous = (siblingCounts.get(key) ?? 0) > 1
    if (ambiguous !== (operation.identityEvidence === 'same-name-positional')) {
      fail('identity', `$.core.operations[${index}].identityEvidence`, 'same-name siblings need positional evidence')
    }
  })
  for (const [group, count] of ambiguityCounts) {
    if (count < 2) fail('identity', '$.core.operations', `singleton ambiguity group ${group}`)
  }
  for (const ordinals of siblingOrdinals.values()) {
    if (ordinals.some((ordinal, index) => ordinal !== index)) {
      fail('order', '$.core.operations', 'same-name sibling ordinals are not dense from zero')
    }
  }
  return operations
}

function validateValueType(value: unknown, path: string): Record<string, any> {
  const valueType = exact(value, KEY.valueType, path)
  if (!includes(TAG.geometryKind, valueType.geometryKind)) {
    fail('type', `${path}.geometryKind`, 'unknown geometry kind')
  }
  if (!includes(TAG.space, valueType.space)) fail('type', `${path}.space`, 'unknown geometry space')
  if ((valueType.geometryKind === 'region' && valueType.space !== 'd2')
    || (['sheet', 'solid', 'solid-set'].includes(valueType.geometryKind) && valueType.space !== 'd3')) {
    fail('type', path, 'geometry kind is incompatible with its ambient space')
  }
  if (!includes(TAG.representation, valueType.representation)) {
    fail('type', `${path}.representation`, 'unknown geometry representation')
  }
  const evidence = record(valueType.evidence, `${path}.evidence`)
  if (typeof evidence.tag !== 'string') {
    fail('schema', `${path}.evidence`, 'expected representation-evidence discriminant')
  }
  if (!includes(TAG.evidence, evidence.tag)) {
    fail('field', `${path}.evidence.tag`, 'unknown representation evidence')
  }
  exact(
    evidence,
    (EVIDENCE_KEY as Readonly<Record<string, readonly string[]>>)[evidence.tag],
    `${path}.evidence`,
  )
  if (evidence.tag === 'representation-preserving') {
    if (valueType.representation === 'certified-approx-brep') {
      fail('type', path, 'certified approximation representation requires certified evidence')
    }
  } else {
    const profile = text(evidence.certificateProfile, `${path}.evidence.certificateProfile`, 96)
    if (!/^[A-Za-z0-9][A-Za-z0-9._:-]{0,95}$/.test(profile)) {
      fail('field', `${path}.evidence.certificateProfile`, 'invalid certificate profile')
    }
    if (typeof evidence.certificatePolicyHash !== 'string'
      || !/^[a-f0-9]{64}$/.test(evidence.certificatePolicyHash)) {
      fail('field', `${path}.evidence.certificatePolicyHash`, 'expected lowercase SHA-256')
    }
    if (valueType.representation !== 'certified-approx-brep') {
      fail('type', path, 'certified evidence requires certified approximation representation')
    }
  }
  return valueType
}

function sameValueType(left: Record<string, any>, right: Record<string, any>): boolean {
  if (left.geometryKind !== right.geometryKind || left.space !== right.space
    || left.representation !== right.representation || left.evidence.tag !== right.evidence.tag) return false
  return left.evidence.tag === 'representation-preserving'
    || (left.evidence.certificateProfile === right.evidence.certificateProfile
      && left.evidence.certificatePolicyHash === right.evidence.certificatePolicyHash)
}

function sameRepresentationEvidence(left: Record<string, any>, right: Record<string, any>): boolean {
  return left.representation === right.representation && left.evidence.tag === right.evidence.tag
    && (left.evidence.tag === 'representation-preserving'
      || (left.evidence.certificateProfile === right.evidence.certificateProfile
        && left.evidence.certificatePolicyHash === right.evidence.certificatePolicyHash))
}

function expectedValueType(contract: string, geometryKind: string, space: string): Record<string, any> {
  return {
    geometryKind,
    space,
    representation: contract === 'legacy/current' ? 'mesh' : 'analytic-brep',
    evidence: { tag: 'representation-preserving' },
  }
}

function earlierNode(value: unknown, path: string, before: number): number {
  const output = integer(value, path, 0, LIMIT.nodes - 1)
  if (output >= before) fail('reference', path, 'node edge must point backward')
  return output
}

type ContinuationAnchors = {
  caller: number
  callerBody: number
  definition: number
  children: number
  expansion: number
}

type ContinuationIndex = {
  staticChildren: ReadonlyMap<number, readonly number[]>
  runtimeChildren: ReadonlyMap<number, readonly number[]>
  nearestModule: Int32Array
}

function sameContinuationBindings(
  left: readonly Record<string, any>[],
  right: readonly Record<string, any>[],
): boolean {
  return left.length === right.length && left.every((slot, index) => (
    slot.name === right[index].name && stableJson(slot.value) === stableJson(right[index].value)
  ))
}

function structuralPrefix(
  prefix: readonly Record<string, any>[],
  path: readonly Record<string, any>[],
): boolean {
  return prefix.length < path.length && prefix.every((segment, index) => (
    segment.kind === path[index].kind
    && segment.name === path[index].name
    && segment.ordinal === path[index].ordinal
  ))
}

/** Recomputes the one admitted caller-body/definition/children continuation. */
function childrenContinuationAnchors(
  expansionIndex: number,
  occurrences: readonly Record<string, any>[],
  operations: readonly Record<string, any>[],
  index: ContinuationIndex,
): ContinuationAnchors | null {
  const expansion = occurrences[expansionIndex]
  if (expansion.parent === null || expansion.staticParent === null) return null
  const children = occurrences[expansion.parent]
  const callerBody = occurrences[expansion.staticParent]
  if (children === undefined || callerBody === undefined) return null
  const expansionOperation = operations[expansion.operation]
  const childrenOperation = operations[children.operation]
  const callerBodyOperation = operations[callerBody.operation]
  if (expansionOperation === undefined || childrenOperation === undefined
    || callerBodyOperation === undefined) return null
  const expansionSegment = expansionOperation.structuralPath.at(-1)
  const childrenSegment = childrenOperation.structuralPath.at(-1)
  const bodySegment = callerBodyOperation.structuralPath.at(-1)
  if (expansionOperation.category !== 'control' || expansionOperation.name !== '$expansion'
    || expansionSegment?.kind !== 'control' || expansionSegment.ordinal !== 0
    || childrenOperation.category !== 'control' || childrenOperation.name !== 'children'
    || childrenSegment?.kind !== 'control'
    || callerBodyOperation.category !== 'control' || callerBodyOperation.name !== '$body'
    || bodySegment?.kind !== 'body' || bodySegment.ordinal !== 0
    || expansionOperation.parent !== callerBody.operation) return null

  const bodyStaticChildren = index.staticChildren.get(callerBody.operation) ?? []
  if (bodyStaticChildren.length !== 1 || bodyStaticChildren[0] !== expansion.operation) return null
  const indexSlot = (slots: readonly Record<string, any>[]): Record<string, any> | null => (
    slots.length === 1 && slots[0].name === '$index' ? slots[0] : null
  )
  const childrenIndex = indexSlot(children.dynamicSlots)
  const expansionIndexSlot = indexSlot(expansion.dynamicSlots)
  if (childrenIndex === null || expansionIndexSlot === null
    || stableJson(childrenIndex.value) !== stableJson(expansionIndexSlot.value)) return null

  const definition = children.parent === null ? -1 : index.nearestModule[children.parent]
  if (definition < 0 || occurrences[definition].parent !== expansion.staticParent) return null
  const caller = callerBody.parent
  if (caller === null) return null
  const callerOccurrence = occurrences[caller]
  const definitionOccurrence = occurrences[definition]
  const callerOperation = operations[callerOccurrence.operation]
  const definitionOperation = operations[definitionOccurrence.operation]
  if (callerOperation === undefined || definitionOperation === undefined) return null

  const callerStaticChildren = index.staticChildren.get(callerOccurrence.operation) ?? []
  const definitionRuntimeChildren = index.runtimeChildren.get(expansion.staticParent) ?? []
  const expansionRuntimeChildren = index.runtimeChildren.get(expansion.parent) ?? []
  if (callerBody.parent !== caller
    || callerStaticChildren.length !== 1 || callerStaticChildren[0] !== callerBody.operation
    || callerOperation.category !== 'module' || callerOperation.structuralPath.at(-1)?.kind !== 'call'
    || definitionOperation.parent !== null || definitionOperation.category !== 'module'
    || definitionOperation.structuralPath.at(-1)?.kind !== 'module'
    || callerOperation.name !== definitionOperation.name
    || !sameContinuationBindings(callerOccurrence.dynamicSlots, definitionOccurrence.dynamicSlots)
    || !structuralPrefix(definitionOperation.structuralPath, childrenOperation.structuralPath)
    || definitionRuntimeChildren.length !== 1 || definitionRuntimeChildren[0] !== definition
    || expansionRuntimeChildren.length !== 1 || expansionRuntimeChildren[0] !== expansionIndex
    || expansion.node !== null || expansion.outputOrdinal !== null || expansion.sceneEntityId !== null) {
    return null
  }
  return { caller, callerBody: expansion.staticParent, definition, children: expansion.parent, expansion: expansionIndex }
}

function isModuleDefinitionActivation(
  definitionIndex: number,
  occurrences: readonly Record<string, any>[],
  operations: readonly Record<string, any>[],
  index: ContinuationIndex,
): boolean {
  const definition = occurrences[definitionIndex]
  if (definition.parent === null) return false
  const body = occurrences[definition.parent]
  if (body === undefined || body.parent === null) return false
  const caller = occurrences[body.parent]
  const definitionOperation = operations[definition.operation]
  const bodyOperation = operations[body.operation]
  const callerOperation = operations[caller.operation]
  if (definitionOperation === undefined || bodyOperation === undefined || callerOperation === undefined) return false
  const callerStaticChildren = index.staticChildren.get(caller.operation) ?? []
  const bodyStaticChildren = index.staticChildren.get(body.operation) ?? []
  const bodyRuntimeChildren = index.runtimeChildren.get(definition.parent) ?? []
  if (definitionOperation.parent !== null || definitionOperation.category !== 'module'
    || definitionOperation.structuralPath.at(-1)?.kind !== 'module'
    || callerOperation.category !== 'module' || callerOperation.structuralPath.at(-1)?.kind !== 'call'
    || callerOperation.name !== definitionOperation.name
    || bodyOperation.parent !== caller.operation || bodyOperation.category !== 'control'
    || bodyOperation.name !== '$body' || bodyOperation.structuralPath.at(-1)?.kind !== 'body'
    || callerStaticChildren.length !== 1 || callerStaticChildren[0] !== body.operation
    || bodyStaticChildren.length !== 1
    || bodyRuntimeChildren.length !== 1 || bodyRuntimeChildren[0] !== definitionIndex
    || !sameContinuationBindings(caller.dynamicSlots, definition.dynamicSlots)) return false
  const expansionOperation = operations[bodyStaticChildren[0]]
  const segment = expansionOperation.structuralPath.at(-1)
  return expansionOperation.parent === body.operation
    && expansionOperation.category === 'control' && expansionOperation.name === '$expansion'
    && segment?.kind === 'control' && segment.ordinal === 0
}

function validateNodes(value: unknown, contract: string): Record<string, any>[] {
  const nodes = array(value, '$.core.nodes', LIMIT.nodes)
    .map((item, index) => record(item, `$.core.nodes[${index}]`))
  nodes.forEach((node, index) => {
    const path = `$.core.nodes[${index}]`
    const kind = text(node.kind, `${path}.kind`, 64)
    const expectedKeys = (NODE_KEY as Readonly<Record<string, readonly string[]>>)[kind]
    if (expectedKeys === undefined) fail('field', `${path}.kind`, 'unknown node tag')
    exact(node, expectedKeys, path)
    if (node.id !== index) fail('order', `${path}.id`, 'ID must equal canonical array position')
    if (contract === 'legacy/current' && kind.endsWith('-analytic')) {
      fail('type', `${path}.kind`, 'legacy contract requires polygonal materialization')
    }
    if (contract === 'openscad-viewer/brep-1' && kind.endsWith('-polygonal')) {
      fail('type', `${path}.kind`, 'B-rep contract requires analytic topology')
    }
    const valueType = validateValueType(node.valueType, `${path}.valueType`)
    const requireValueType = (expected: Record<string, any>): void => {
      if (!sameValueType(valueType, expected)) fail('type', `${path}.valueType`, 'value type does not match node transition')
    }
    const inputNode = (field: string): Record<string, any> => {
      const reference = earlierNode(node[field], `${path}.${field}`, index)
      return nodes[reference]
    }

    switch (kind) {
      case 'box':
        requireValueType(expectedValueType(contract, contract === 'legacy/current' ? 'solid-set' : 'solid', 'd3'))
        if (tuple(node.size, 3, `${path}.size`).some(item => item <= 0)) fail('number', `${path}.size`, 'sizes must be positive')
        boolean(node.center, `${path}.center`)
        break
      case 'sphere-analytic':
      case 'sphere-polygonal':
        requireValueType(expectedValueType(contract, contract === 'legacy/current' ? 'solid-set' : 'solid', 'd3'))
        positive(node.radius, `${path}.radius`)
        if (kind === 'sphere-polygonal') integer(node.radialSegments, `${path}.radialSegments`, 4, 1_000_000)
        break
      case 'cylinder-analytic':
      case 'cylinder-polygonal': {
        requireValueType(expectedValueType(contract, contract === 'legacy/current' ? 'solid-set' : 'solid', 'd3'))
        positive(node.height, `${path}.height`)
        const bottom = positive(node.radiusBottom, `${path}.radiusBottom`, true)
        const top = positive(node.radiusTop, `${path}.radiusTop`, true)
        if (bottom === 0 && top === 0) fail('number', path, 'both radii cannot be zero')
        boolean(node.center, `${path}.center`)
        if (kind === 'cylinder-polygonal') integer(node.radialSegments, `${path}.radialSegments`, 3, 1_000_000)
        break
      }
      case 'polyhedron': {
        requireValueType(expectedValueType(contract, 'solid-set', 'd3'))
        const vertices = array(node.vertices, `${path}.vertices`, 750_000)
        const triangles = array(node.triangles, `${path}.triangles`, 750_000)
        const legacyZeroPolyhedron = contract === 'legacy/current'
          && vertices.length === 0
          && triangles.length === 0
        if (vertices.length < 4 && !legacyZeroPolyhedron) {
          fail('type', `${path}.vertices`, 'solid needs four vertices or the exact legacy zero/zero form')
        }
        vertices.forEach((vertex, vertexIndex) => tuple(vertex, 3, `${path}.vertices[${vertexIndex}]`))
        triangles.forEach((triangle, triangleIndex) => {
          const indices = array(triangle, `${path}.triangles[${triangleIndex}]`, 3)
          if (indices.length !== 3) fail('field', `${path}.triangles[${triangleIndex}]`, 'triangle arity')
          const parsed = indices.map((item, component) => integer(item, `${path}.triangles[${triangleIndex}][${component}]`, 0, vertices.length - 1))
          if (contract !== 'legacy/current' && new Set(parsed).size !== 3) {
            fail('type', `${path}.triangles[${triangleIndex}]`, 'triangle repeats vertex')
          }
        })
        break
      }
      case 'rectangle':
        requireValueType(expectedValueType(contract, 'region', 'd2'))
        if (tuple(node.size, 2, `${path}.size`).some(item => item <= 0)) fail('number', `${path}.size`, 'sizes must be positive')
        boolean(node.center, `${path}.center`)
        break
      case 'circle-analytic':
      case 'circle-polygonal':
        requireValueType(expectedValueType(contract, 'region', 'd2'))
        positive(node.radius, `${path}.radius`)
        if (kind === 'circle-polygonal') integer(node.radialSegments, `${path}.radialSegments`, 3, 1_000_000)
        break
      case 'polygon': {
        requireValueType(expectedValueType(contract, 'region', 'd2'))
        if (node.fillRule !== 'even-odd') fail('field', `${path}.fillRule`, 'unknown fill rule')
        const rings = array(node.rings, `${path}.rings`, 100_000)
        if (rings.length === 0) fail('field', `${path}.rings`, 'polygon needs a ring')
        rings.forEach((ring, ringIndex) => {
          const points = array(ring, `${path}.rings[${ringIndex}]`, 1_000_000)
          if (contract !== 'legacy/current' && points.length < 3) {
            fail('field', `${path}.rings[${ringIndex}]`, 'ring needs three points')
          }
          points.forEach((point, pointIndex) => tuple(point, 2, `${path}.rings[${ringIndex}][${pointIndex}]`))
        })
        break
      }
      case 'transform': {
        requireValueType(inputNode('input').valueType)
        const matrix = tuple(node.matrix, 16, `${path}.matrix`)
        if (matrix[3] !== 0 || matrix[7] !== 0 || matrix[11] !== 0 || matrix[15] !== 1) {
          fail('type', `${path}.matrix`, 'matrix is not affine column-major')
        }
        const determinant = matrix[0] * (matrix[5] * matrix[10] - matrix[9] * matrix[6])
          - matrix[4] * (matrix[1] * matrix[10] - matrix[9] * matrix[2])
          + matrix[8] * (matrix[1] * matrix[6] - matrix[5] * matrix[2])
        if (contract === 'openscad-viewer/brep-1'
          && (!Number.isFinite(determinant) || determinant === 0)) {
          fail('type', `${path}.matrix`, 'singular B-rep transform')
        }
        break
      }
      case 'boolean':
      case 'hull': {
        if (kind === 'boolean' && !['union', 'intersection', 'difference'].includes(node.operation)) {
          fail('field', `${path}.operation`, 'unknown Boolean operation')
        }
        const inputs = array(node.inputs, `${path}.inputs`, LIMIT.nodes)
        if (inputs.length < 2) fail('type', `${path}.inputs`, 'requires at least two inputs')
        const references = inputs.map((item, inputIndex) => earlierNode(item, `${path}.inputs[${inputIndex}]`, index))
        const firstType = nodes[references[0]].valueType
        references.forEach((reference, inputIndex) => {
          const candidate = nodes[reference].valueType
          if (candidate.space !== firstType.space || !sameRepresentationEvidence(candidate, firstType)) {
            fail('type', `${path}.inputs[${inputIndex}]`, 'operands must share space, representation, and evidence')
          }
          const admitted = firstType.space === 'd2'
            ? candidate.geometryKind === 'region'
            : candidate.geometryKind === 'solid' || candidate.geometryKind === 'solid-set'
          if (!admitted) fail('type', `${path}.inputs[${inputIndex}]`, 'operand geometry kind is not admitted')
        })
        requireValueType({
          geometryKind: firstType.space === 'd2' ? 'region' : 'solid-set',
          space: firstType.space,
          representation: firstType.representation,
          evidence: firstType.evidence,
        })
        break
      }
      case 'linear-extrude': {
        const inputType = inputNode('input').valueType
        if (inputType.geometryKind !== 'region' || inputType.space !== 'd2') {
          fail('type', `${path}.input`, 'linear extrusion consumes Region/d2')
        }
        requireValueType({ ...inputType, geometryKind: 'solid-set', space: 'd3' })
        positive(node.height, `${path}.height`)
        number(node.twistDegrees, `${path}.twistDegrees`)
        integer(node.slices, `${path}.slices`, 0, 1_000_000)
        if (contract === 'openscad-viewer/brep-1'
          && tuple(node.scale, 2, `${path}.scale`).some(item => item === 0)) {
          fail('number', `${path}.scale`, 'B-rep extrusion scale cannot be zero')
        } else if (contract === 'legacy/current') {
          tuple(node.scale, 2, `${path}.scale`)
        }
        boolean(node.center, `${path}.center`)
        break
      }
      case 'rotate-extrude-analytic':
      case 'rotate-extrude-polygonal': {
        const inputType = inputNode('input').valueType
        if (inputType.geometryKind !== 'region' || inputType.space !== 'd2') {
          fail('type', `${path}.input`, 'rotate extrusion consumes Region/d2')
        }
        requireValueType({ ...inputType, geometryKind: 'solid-set', space: 'd3' })
        const angle = contract === 'legacy/current'
          ? number(node.angleDegrees, `${path}.angleDegrees`)
          : positive(node.angleDegrees, `${path}.angleDegrees`)
        if (contract === 'openscad-viewer/brep-1' && angle > 360) {
          fail('number', `${path}.angleDegrees`, 'angle exceeds 360')
        }
        if (kind === 'rotate-extrude-polygonal') integer(node.radialSegments, `${path}.radialSegments`, 3, 1_000_000)
        break
      }
      case 'projection': {
        const inputType = inputNode('input').valueType
        if (inputType.space !== 'd3'
          || (inputType.geometryKind !== 'solid' && inputType.geometryKind !== 'solid-set')) {
          fail('type', `${path}.input`, 'projection consumes Solid|SolidSet/d3')
        }
        requireValueType({ ...inputType, geometryKind: 'region', space: 'd2' })
        boolean(node.cut, `${path}.cut`)
        break
      }
      case 'offset': {
        const inputType = inputNode('input').valueType
        if (inputType.geometryKind !== 'region' || inputType.space !== 'd2') {
          fail('type', `${path}.input`, 'offset consumes Region/d2')
        }
        requireValueType(inputType)
        number(node.distance, `${path}.distance`)
        break
      }
    }
  })
  return nodes
}

function validateOccurrences(
  value: unknown,
  operations: Record<string, any>[],
  nodes: Record<string, any>[],
): Record<string, any>[] {
  const occurrences = array(value, '$.core.occurrences', LIMIT.occurrences)
    .map((item, index) => record(item, `$.core.occurrences[${index}]`))
  const staticChildren = new Map<number, number[]>()
  operations.forEach((operation, index) => {
    if (operation.parent === null) return
    const children = staticChildren.get(operation.parent) ?? []
    children.push(index)
    staticChildren.set(operation.parent, children)
  })
  const runtimeChildren = new Map<number, number[]>()
  const nearestModule = new Int32Array(occurrences.length)
  nearestModule.fill(-1)
  occurrences.forEach((occurrence, index) => {
    const parent = Number.isSafeInteger(occurrence.parent)
      && occurrence.parent >= 0 && occurrence.parent < index ? occurrence.parent : null
    if (parent !== null) {
      const children = runtimeChildren.get(parent) ?? []
      children.push(index)
      runtimeChildren.set(parent, children)
    }
    const operation = Number.isSafeInteger(occurrence.operation)
      && occurrence.operation >= 0 && occurrence.operation < operations.length
      ? operations[occurrence.operation] : null
    nearestModule[index] = operation?.structuralPath.at(-1)?.kind === 'module'
      ? index
      : parent === null ? -1 : nearestModule[parent]
  })
  const continuationIndex: ContinuationIndex = { staticChildren, runtimeChildren, nearestModule }
  const continuations = new Map<number, ContinuationAnchors>()
  const sceneIds = new Set<string>()
  const occurrenceGroups = new Map<string, number[]>()
  const duplicateSlotGroups = new Map<string, Map<number, string>>()
  occurrences.forEach((occurrence, index) => {
    const path = `$.core.occurrences[${index}]`
    exact(occurrence, KEY.occurrence, path)
    if (occurrence.id !== index) fail('order', `${path}.id`, 'ID must equal array position')
    const operation = integer(occurrence.operation, `${path}.operation`, 0, operations.length - 1)
    const parent = occurrence.parent === null
      ? null
      : integer(occurrence.parent, `${path}.parent`, 0, index - 1)
    const staticParentOperation = operations[operation].parent
    const staticParent = occurrence.staticParent === null
      ? null
      : integer(occurrence.staticParent, `${path}.staticParent`, 0, index - 1)
    let crossedOpaqueFrame = false
    if (staticParentOperation === null) {
      if (staticParent !== null) {
        fail('identity', `${path}.staticParent`, 'root static operation requires null staticParent')
      }
    } else {
      if (staticParent === null || occurrences[staticParent].operation !== staticParentOperation) {
        fail('identity', `${path}.staticParent`, 'staticParent does not instantiate the static parent operation')
      }
      let cursor = parent
      while (cursor !== staticParent) {
        if (cursor === null) {
          fail('identity', `${path}.staticParent`, 'staticParent is not on the runtime ancestor chain')
        }
        const skipped = occurrences[cursor]
        if (skipped.operation === staticParentOperation) {
          fail('identity', `${path}.staticParent`, 'staticParent is not the nearest matching ancestor')
        }
        const category = operations[skipped.operation].category
        if (category !== 'control' && category !== 'module') {
          crossedOpaqueFrame = true
        }
        cursor = skipped.parent
      }
    }
    const slots = array(occurrence.dynamicSlots, `${path}.dynamicSlots`, 1_000)
    const names = new Set<string>()
    slots.forEach((item, slotIndex) => {
      const slotPath = `${path}.dynamicSlots[${slotIndex}]`
      const slot = exact(item, KEY.dynamicSlot, slotPath)
      const name = text(slot.name, `${slotPath}.name`, 128)
      if (names.has(name)) fail('identity', `${slotPath}.name`, 'duplicate dynamic slot')
      names.add(name)
      validateIdentityValue(slot.value, `${slotPath}.value`)
      integer(slot.duplicateOrdinal, `${slotPath}.duplicateOrdinal`, 0, 1_000_000)
    })
    if (staticParentOperation !== null) {
      if (operations[operation].name === '$expansion') {
        const anchors = childrenContinuationAnchors(index, occurrences, operations, continuationIndex)
        if (anchors === null) {
          fail('identity', `${path}.parent`, '$expansion is not the exact caller children continuation')
        }
        continuations.set(index, anchors)
      } else if (crossedOpaqueFrame) {
        fail('identity', `${path}.parent`, 'only control/module frames may separate parent from staticParent')
      }
    }
    const parentId = parent === null ? null : occurrences[parent].occurrenceId
    const staticParentId = staticParent === null ? null : occurrences[staticParent].occurrenceId
    const expectedId = referenceSemanticOccurrenceIdV1(
      parentId,
      staticParentId,
      operations[operation].operationId,
      slots,
    )
    if (occurrence.occurrenceId !== expectedId) fail('identity', `${path}.occurrenceId`, 'digest mismatch')
    const group = occurrenceGroups.get(expectedId) ?? []
    if (group.length > 0) {
      const first = occurrences[group[0]]
      if (first.operation !== operation || first.parent !== parent || first.staticParent !== staticParent
        || stableJson(first.dynamicSlots) !== stableJson(slots)) {
        fail('identity', path, 'shared occurrence identity describes different evaluation')
      }
    }
    group.push(index)
    occurrenceGroups.set(expectedId, group)
    slots.forEach((slot, slotIndex) => {
      const duplicateKey = stableJson([
        parent,
        operation,
        slotIndex,
        slot.name,
        slot.value,
      ])
      const ordinals = duplicateSlotGroups.get(duplicateKey) ?? new Map<number, string>()
      const prior = ordinals.get(slot.duplicateOrdinal)
      if (prior !== undefined && prior !== expectedId) {
        fail('identity', `${path}.dynamicSlots[${slotIndex}].duplicateOrdinal`, 'duplicate ordinal aliases equal dynamic values')
      }
      if (prior === undefined && slot.duplicateOrdinal !== ordinals.size) {
        fail('order', `${path}.dynamicSlots[${slotIndex}].duplicateOrdinal`, 'duplicate ordinals must be first-seen dense in language order')
      }
      ordinals.set(slot.duplicateOrdinal, expectedId)
      duplicateSlotGroups.set(duplicateKey, ordinals)
    })

    if (occurrence.node === null) {
      if (occurrence.outputOrdinal !== null || occurrence.sceneEntityId !== null) {
        fail('identity', path, 'non-producing occurrence owns output identity')
      }
      return
    }
    integer(occurrence.node, `${path}.node`, 0, nodes.length - 1)
    if (occurrence.outputOrdinal === null) {
      fail('identity', `${path}.outputOrdinal`, 'node-bearing occurrence must be a dense output row')
    }
    const ordinal = integer(occurrence.outputOrdinal, `${path}.outputOrdinal`, 0, 1_000_000)
    const expectedScene = referenceSemanticSceneEntityIdV1(expectedId, ordinal)
    if (occurrence.sceneEntityId !== expectedScene) fail('identity', `${path}.sceneEntityId`, 'scene digest mismatch')
    if (sceneIds.has(expectedScene)) fail('identity', `${path}.sceneEntityId`, 'scene identity collision')
    sceneIds.add(expectedScene)
  })
  occurrences.forEach((occurrence, index) => {
    for (const field of ['parent', 'staticParent'] as const) {
      const reference = occurrence[field]
      if (reference === null) continue
      const referencedId = occurrences[reference].occurrenceId
      if (occurrenceGroups.get(referencedId)?.[0] !== reference) {
        fail('order', `$.core.occurrences[${index}].${field}`, 'occurrence references must target the canonical first row')
      }
    }
  })
  for (const indexes of occurrenceGroups.values()) {
    const outputs = indexes.filter(index => occurrences[index].outputOrdinal !== null)
    if (indexes.length > 1 && outputs.length !== indexes.length) {
      fail('identity', `$.core.occurrences[${indexes[1]}]`, 'repeated occurrence identity is reserved for output slots')
    }
    outputs.forEach((index, ordinal) => {
      if (occurrences[index].outputOrdinal !== ordinal) {
        fail('order', `$.core.occurrences[${index}].outputOrdinal`, 'output slots are not dense from zero')
      }
    })
  }
  for (const ordinals of duplicateSlotGroups.values()) {
    const ordered = [...ordinals.keys()].sort((left, right) => left - right)
    if (ordered.some((ordinal, index) => ordinal !== index)) {
      fail('order', '$.core.occurrences', 'dynamic duplicate ordinals are not dense from zero')
    }
  }
  occurrences.forEach((occurrence, index) => {
    if (operations[occurrence.operation].parent !== null || occurrence.parent === null) return
    if (!isModuleDefinitionActivation(index, occurrences, operations, continuationIndex)) {
      fail('identity', `$.core.occurrences[${index}].parent`, 'static-root occurrence is injected outside its matching module activation')
    }
  })
  for (const anchors of continuations.values()) {
    const expansionOperation = operations[occurrences[anchors.expansion].operation]
    const expandedChildren = runtimeChildren.get(anchors.expansion) ?? []
    for (const child of expandedChildren) {
      const occurrence = occurrences[child]
      const operation = operations[occurrence.operation]
      if (occurrence.staticParent !== anchors.expansion
        || !structuralPrefix(expansionOperation.structuralPath, operation.structuralPath)) {
        fail('identity', `$.core.occurrences[${child}].parent`, 'expanded child escapes its caller-child static subtree')
      }
    }
  }
  return occurrences
}

function nodeInputs(node: Record<string, any>): number[] {
  if (['transform', 'linear-extrude', 'rotate-extrude-analytic', 'rotate-extrude-polygonal', 'projection', 'offset'].includes(node.kind)) {
    return [node.input]
  }
  if (node.kind === 'boolean' || node.kind === 'hull') return node.inputs
  return []
}

function occurrenceRoot(index: number, occurrences: Record<string, any>[]): number {
  let cursor = index
  while (occurrences[cursor].parent !== null) cursor = occurrences[cursor].parent
  return cursor
}

function occurrenceIsDescendantOf(
  candidate: number,
  ancestor: number,
  occurrences: Record<string, any>[],
): boolean {
  let cursor: number | null = candidate
  while (cursor !== null) {
    if (cursor === ancestor) return true
    cursor = occurrences[cursor].parent
  }
  return false
}

function identityPreservingNodeLineage(
  outputNode: number,
  identityNode: number,
  nodes: Record<string, any>[],
): boolean {
  let cursor = outputNode
  while (cursor !== identityNode) {
    const node = nodes[cursor]
    if (node.kind !== 'transform') return false
    cursor = node.input
  }
  return true
}

function validateResult(
  value: unknown,
  nodes: Record<string, any>[],
  occurrences: Record<string, any>[],
): number[] {
  const firstRowByOccurrenceId = new Map<string, number>()
  const canonicalOccurrenceRows = occurrences.map((occurrence, index) => {
    const first = firstRowByOccurrenceId.get(occurrence.occurrenceId)
    if (first !== undefined) return first
    firstRowByOccurrenceId.set(occurrence.occurrenceId, index)
    return index
  })
  const result = record(value, '$.core.result')
  if (typeof result.tag !== 'string') fail('schema', '$.core.result', 'expected result discriminant')
  if (!includes(TAG.result, result.tag)) fail('field', '$.core.result.tag', 'unknown result tag')
  const roots: number[] = []
  const identityOwners = new Set<number>()
  let priorIdentity = -1

  const output = (item: unknown, path: string): void => {
    const reference = exact(item, KEY.output, path)
    const node = integer(reference.node, `${path}.node`, 0, nodes.length - 1)
    const producer = integer(reference.producerOccurrence, `${path}.producerOccurrence`, 0, occurrences.length - 1)
    const identity = integer(reference.identityOccurrence, `${path}.identityOccurrence`, 0, occurrences.length - 1)
    if (occurrences[producer].node !== node) fail('identity', path, 'producer does not materialize output node')
    if (occurrences[identity].sceneEntityId === null) fail('identity', path, 'identity occurrence has no scene identity')
    const identityNode = occurrences[identity].node
    const canonicalProducer = canonicalOccurrenceRows[producer]
    const identityIsInProducingBranch = occurrenceIsDescendantOf(identity, producer, occurrences)
      || (
        occurrences[canonicalProducer].occurrenceId === occurrences[producer].occurrenceId
        && occurrenceIsDescendantOf(identity, canonicalProducer, occurrences)
      )
    if (identityNode === null || !identityPreservingNodeLineage(node, identityNode, nodes)
      || occurrenceRoot(producer, occurrences) !== occurrenceRoot(identity, occurrences)
      || !identityIsInProducingBranch) {
      fail('identity', `${path}.identityOccurrence`, 'identity is outside producing branch or node lineage')
    }
    if (identity <= priorIdentity) fail('order', `${path}.identityOccurrence`, 'root outputs are not in occurrence order')
    priorIdentity = identity
    if (identityOwners.has(identity)) fail('identity', `${path}.identityOccurrence`, 'scene owner reused by multiple outputs')
    identityOwners.add(identity)
    const color = tuple(reference.color, 4, `${path}.color`)
    if (color.some(channel => channel < 0 || channel > 1)) fail('number', `${path}.color`, 'channel outside [0,1]')
    roots.push(node)
  }

  if (result.tag === 'empty') {
    exact(result, ['tag', 'type'], '$.core.result')
    if (result.type !== 'never') fail('type', '$.core.result.type', 'empty result must be never')
  } else if (result.tag === 'single') {
    exact(result, ['tag', 'item'], '$.core.result')
    output(result.item, '$.core.result.item')
  } else {
    exact(result, ['tag', 'items'], '$.core.result')
    const items = array(result.items, '$.core.result.items', LIMIT.outputs)
    if (items.length < 2) fail('type', '$.core.result.items', 'multi needs at least two outputs')
    items.forEach((item, index) => output(item, `$.core.result.items[${index}]`))
  }
  return roots
}

type ReferenceExecutionEffect = {
  tag: 'legacy-difference-cutters'
  root: number
  ownerOccurrence: number
}

type ReferenceExecutionTerminal = null | {
  tag: 'legacy-language-error'
  occurrence: number
  diagnosticTemplate: number
  prefixFrontier: Array<{ root: number; ownerOccurrence: number | null }>
}

type ReferenceExecution = {
  evaluationOrder: number[]
  discardedEffects: ReferenceExecutionEffect[]
  terminal: ReferenceExecutionTerminal
}

function validateDiagnosticTemplates(
  value: unknown,
  operations: Record<string, any>[],
): Record<string, any>[] {
  return array(value, '$.core.diagnosticTemplates', LIMIT.diagnostics).map((item, index) => {
    const path = `$.core.diagnosticTemplates[${index}]`
    const template = exact(item, KEY.diagnosticTemplate, path)
    if (template.id !== index) fail('order', `${path}.id`, 'ID must equal canonical array position')
    const code = text(template.code, `${path}.code`, 64)
    if (!/^[A-Z][A-Z0-9_]{0,63}$/.test(code)) fail('field', `${path}.code`, 'invalid diagnostic code')
    if (!['info', 'warning', 'error'].includes(template.severity)) {
      fail('field', `${path}.severity`, 'invalid severity')
    }
    if (template.severity === 'error' && code !== 'LEGACY_LANGUAGE_ERROR') {
      fail('field', `${path}.code`, 'error severity is reserved for the frozen legacy terminal')
    }
    if (template.operation !== null) {
      if (operations.length === 0) fail('reference', `${path}.operation`, 'diagnostic references absent operation')
      integer(template.operation, `${path}.operation`, 0, operations.length - 1)
    }
    const names = new Set<string>()
    array(template.arguments, `${path}.arguments`, 128).forEach((argumentValue, argumentIndex) => {
      const argumentPath = `${path}.arguments[${argumentIndex}]`
      const argument = exact(argumentValue, KEY.diagnosticArgument, argumentPath)
      const name = text(argument.name, `${argumentPath}.name`, 128)
      if (names.has(name)) fail('order', `${argumentPath}.name`, 'duplicate diagnostic argument')
      names.add(name)
      validateIdentityValue(argument.value, `${argumentPath}.value`)
    })
    return template
  })
}

function nodesReachableFrom(nodes: Record<string, any>[], roots: readonly number[]): Uint8Array {
  const reached = new Uint8Array(nodes.length)
  const pending = [...roots]
  while (pending.length > 0) {
    const node = pending.pop()!
    if (reached[node]) continue
    reached[node] = 1
    for (const input of nodeInputs(nodes[node])) pending.push(input)
  }
  return reached
}

function operationMaterializesNode(operation: Record<string, any>, node: Record<string, any>): boolean {
  if (node.kind === 'box') return operation.name === 'cube'
  if (node.kind.startsWith('sphere-')) return operation.name === 'sphere'
  if (node.kind.startsWith('cylinder-')) return operation.name === 'cylinder'
  if (node.kind === 'polyhedron') return operation.name === 'polyhedron'
  if (node.kind === 'rectangle') return operation.name === 'square'
  if (node.kind.startsWith('circle-')) return operation.name === 'circle'
  if (node.kind === 'polygon') return operation.name === 'polygon'
  if (node.kind === 'transform') return ['translate', 'rotate', 'scale', 'mirror', 'multmatrix'].includes(operation.name)
  if (node.kind === 'boolean') return operation.name === node.operation
  if (node.kind === 'hull') return operation.name === 'hull'
  if (node.kind === 'linear-extrude') return operation.name === 'linear_extrude'
  if (node.kind.startsWith('rotate-extrude-')) return operation.name === 'rotate_extrude'
  if (node.kind === 'projection') return operation.name === 'projection'
  if (node.kind === 'offset') return operation.name === 'offset'
  return false
}

/** Independent, bounded validation of the hash-covered execution witness. */
function validateExecution(
  value: unknown,
  nodes: Record<string, any>[],
  operations: Record<string, any>[],
  occurrences: Record<string, any>[],
  outputRoots: number[],
  templates: Record<string, any>[],
  contract: string,
): ReferenceExecution {
  const execution = exact(value, KEY.execution, '$.core.execution')
  if (execution.version !== 'semantic-execution-v2') {
    fail('version', '$.core.execution.version', 'unknown execution witness version')
  }
  const rawOrder = array(execution.evaluationOrder, '$.core.execution.evaluationOrder', LIMIT.nodes)
  if (rawOrder.length !== nodes.length) {
    fail('graph', '$.core.execution.evaluationOrder', 'order must be a complete node permutation')
  }
  const position = new Int32Array(nodes.length)
  position.fill(-1)
  const evaluationOrder = rawOrder.map((rawNode, index) => {
    if (nodes.length === 0) fail('reference', `$.core.execution.evaluationOrder[${index}]`, 'node set is empty')
    const node = integer(rawNode, `$.core.execution.evaluationOrder[${index}]`, 0, nodes.length - 1)
    if (position[node] !== -1) fail('order', `$.core.execution.evaluationOrder[${index}]`, 'duplicate scheduled node')
    if (nodeInputs(nodes[node]).some(input => position[input] === -1)) {
      fail('graph', `$.core.execution.evaluationOrder[${index}]`, 'consumer precedes a dependency')
    }
    position[node] = index
    if (node !== index) {
      fail('order', `$.core.execution.evaluationOrder[${index}]`, 'node storage is not the exact authored kernel order')
    }
    return node
  })

  const canonicalRows = new Map<string, number>()
  occurrences.forEach((occurrence, index) => {
    if (!canonicalRows.has(occurrence.occurrenceId)) canonicalRows.set(occurrence.occurrenceId, index)
  })
  const rootsSeen = new Set<number>()
  let priorEffectPosition = -1
  const discardedEffects = array(
    execution.discardedEffects,
    '$.core.execution.discardedEffects',
    LIMIT.discardedEffects,
  ).map((rawEffect, index): ReferenceExecutionEffect => {
    const path = `$.core.execution.discardedEffects[${index}]`
    const effect = exact(rawEffect, KEY.discardedEffect, path)
    if (effect.tag !== 'legacy-difference-cutters') fail('field', `${path}.tag`, 'unknown effect tag')
    if (contract !== 'legacy/current') fail('type', path, 'legacy discarded effects are forbidden in B-rep')
    if (nodes.length === 0) fail('reference', `${path}.root`, 'effect references an absent node')
    const root = integer(effect.root, `${path}.root`, 0, nodes.length - 1)
    if (rootsSeen.has(root)) fail('order', `${path}.root`, 'effect root is duplicated')
    if (position[root] <= priorEffectPosition) fail('order', `${path}.root`, 'effects do not follow evaluation order')
    rootsSeen.add(root)
    priorEffectPosition = position[root]
    if (occurrences.length === 0) fail('reference', `${path}.ownerOccurrence`, 'effect owner is absent')
    const ownerOccurrence = integer(effect.ownerOccurrence, `${path}.ownerOccurrence`, 0, occurrences.length - 1)
    if (canonicalRows.get(occurrences[ownerOccurrence].occurrenceId) !== ownerOccurrence) {
      fail('order', `${path}.ownerOccurrence`, 'effect owner is not its canonical occurrence row')
    }
    return { tag: 'legacy-difference-cutters', root, ownerOccurrence }
  })

  let terminal: ReferenceExecutionTerminal = null
  if (execution.terminal !== null) {
    const path = '$.core.execution.terminal'
    const rawTerminal = exact(execution.terminal, KEY.terminal, path)
    if (rawTerminal.tag !== 'legacy-language-error') fail('field', `${path}.tag`, 'unknown terminal tag')
    if (contract !== 'legacy/current') fail('type', path, 'legacy terminal is forbidden in B-rep')
    if (outputRoots.length !== 0) fail('type', '$.core.result', 'terminal execution cannot publish roots')
    if (templates.length === 0) fail('reference', `${path}.diagnosticTemplate`, 'terminal template is absent')
    const diagnosticTemplate = integer(
      rawTerminal.diagnosticTemplate,
      `${path}.diagnosticTemplate`,
      0,
      templates.length - 1,
    )
    if (templates[diagnosticTemplate].severity !== 'error') {
      fail('reference', `${path}.diagnosticTemplate`, 'terminal must select an error template')
    }
    if (occurrences.length === 0) fail('reference', `${path}.occurrence`, 'terminal occurrence is absent')
    const occurrence = integer(rawTerminal.occurrence, `${path}.occurrence`, 0, occurrences.length - 1)
    if (canonicalRows.get(occurrences[occurrence].occurrenceId) !== occurrence) {
      fail('order', `${path}.occurrence`, 'terminal must name a canonical occurrence row')
    }
    const interrupted = occurrences[occurrence]
    const interruptedId = interrupted.occurrenceId
    const isRuntimeDescendant = (candidate: number): boolean => {
      let cursor: number | null = candidate
      while (cursor !== null) {
        if (cursor === occurrence) return true
        cursor = occurrences[cursor].parent
      }
      return false
    }
    // This proves only a self-consistent active runtime subtree. Establishing
    // that it represents the source's first error still requires the trusted
    // lowerer; a structural artifact cannot make that source-level claim.
    for (let index = occurrence + 1; index < occurrences.length; index++) {
      if (occurrences[index].occurrenceId === interruptedId || isRuntimeDescendant(index)) continue
      fail('order', `${path}.occurrence`, 'later sibling lies outside the interrupted runtime subtree')
    }
    const terminalTemplate = templates[diagnosticTemplate]
    if (terminalTemplate.operation !== interrupted.operation) {
      fail('identity', `${path}.diagnosticTemplate`, 'terminal template does not name the interrupted activation')
    }
    if (terminalTemplate.code !== 'LEGACY_LANGUAGE_ERROR'
      || terminalTemplate.severity !== 'error'
      || terminalTemplate.arguments.length !== 2
      || terminalTemplate.arguments[0].name !== 'errorName'
      || terminalTemplate.arguments[0].value.tag !== 'string'
      || !['OpenSCADParseError', 'TypeError'].includes(terminalTemplate.arguments[0].value.value)
      || terminalTemplate.arguments[1].name !== 'detailSha256'
      || terminalTemplate.arguments[1].value.tag !== 'string'
      || !/^[a-f0-9]{64}$/.test(terminalTemplate.arguments[1].value.value)) {
      fail('identity', `${path}.diagnosticTemplate`, 'terminal template does not have the exact v2 error identity')
    }
    if (discardedEffects.length !== 0) {
      fail('field', '$.core.execution.discardedEffects', 'terminal prefix subsumes discarded work')
    }
    if (templates.some((template, index) => template.severity === 'error' && index !== diagnosticTemplate)) {
      fail('reference', `${path}.diagnosticTemplate`, 'unreferenced error template is forbidden')
    }
    const frontierRoots = new Set<number>()
    const prefixFrontier = array(
      rawTerminal.prefixFrontier,
      `${path}.prefixFrontier`,
      LIMIT.nodes,
    ).map((rawEntry, index) => {
      const entryPath = `${path}.prefixFrontier[${index}]`
      const entry = exact(rawEntry, KEY.terminalPrefix, entryPath)
      if (nodes.length === 0) fail('reference', `${entryPath}.root`, 'prefix node is absent')
      const root = integer(entry.root, `${entryPath}.root`, 0, nodes.length - 1)
      if (frontierRoots.has(root)) fail('order', `${entryPath}.root`, 'prefix root is duplicated')
      frontierRoots.add(root)
      let ownerOccurrence: number | null = null
      if (entry.ownerOccurrence !== null) {
        if (occurrences.length === 0) fail('reference', `${entryPath}.ownerOccurrence`, 'prefix owner is absent')
        ownerOccurrence = integer(entry.ownerOccurrence, `${entryPath}.ownerOccurrence`, 0, occurrences.length - 1)
        if (occurrences[ownerOccurrence].node !== root) {
          fail('identity', `${entryPath}.ownerOccurrence`, 'prefix owner does not materialize its root')
        }
      }
      const materializer = occurrences.findIndex(occurrence => occurrence.node === root
        && operationMaterializesNode(operations[occurrence.operation], nodes[root]))
      const expectedOwner = materializer < 0 ? null : materializer
      if (ownerOccurrence !== expectedOwner) {
        fail('identity', `${entryPath}.ownerOccurrence`, 'prefix owner is not the exact node producer')
      }
      return { root, ownerOccurrence }
    })
    const consumed = new Uint8Array(nodes.length)
    nodes.forEach(node => nodeInputs(node).forEach(input => { consumed[input] = 1 }))
    const maximal = evaluationOrder.filter(node => consumed[node] === 0)
    if (maximal.length !== prefixFrontier.length
      || maximal.some((root, index) => root !== prefixFrontier[index].root)) {
      fail('graph', `${path}.prefixFrontier`, 'prefix is not the ordered maximal completed frontier')
    }
    terminal = { tag: 'legacy-language-error', occurrence, diagnosticTemplate, prefixFrontier }
  } else if (templates.some(template => template.severity === 'error')) {
    fail('reference', '$.core.execution.terminal', 'error template has no terminal witness')
  }

  const published = nodesReachableFrom(nodes, outputRoots)
  for (const effect of discardedEffects) {
    if (published[effect.root]) fail('graph', '$.core.execution.discardedEffects', 'discarded effect is publishable')
  }
  const coverageRoots = terminal === null
    ? [...outputRoots, ...discardedEffects.map(effect => effect.root)]
    : terminal.prefixFrontier.map(entry => entry.root)
  const covered = nodesReachableFrom(nodes, coverageRoots)
  if (covered.some(flag => flag === 0)) {
    fail('graph', '$.core.nodes', 'node is hidden from result, effects, and terminal prefix')
  }
  return { evaluationOrder, discardedEffects, terminal }
}

type ProductionItem = {
  node: number
  row: number
  identityOwner: number
  group: number
  directChild: number
}

type ScheduleEvent =
  | { tag: 'group'; group: number }
  | { tag: 'node'; node: number }

type ProductionGroup = {
  outputRows: number[]
  outputItems: ProductionItem[]
  frontier: ProductionItem[]
  terminalItems: ProductionItem[]
  buckets: ProductionItem[][]
  schedule: ScheduleEvent[]
  scheduleBuckets: ScheduleEvent[][]
  transparent: boolean
}

function productionRule(
  operation: Record<string, any>,
  allowInterruptedUnsupported = false,
): string {
  const pair = `${String(operation.category)}:${String(operation.name)}`
  const last = operation.structuralPath[operation.structuralPath.length - 1]
  const prior = operation.structuralPath[operation.structuralPath.length - 2]
  const authoredCall = last.kind === 'call'
  if (operation.category === 'geometry' && authoredCall
    && ['cube', 'sphere', 'cylinder', 'polyhedron', 'square', 'circle', 'polygon'].includes(operation.name)) return 'primitive'
  if (operation.category === 'transform' && authoredCall
    && ['translate', 'rotate', 'scale', 'mirror', 'multmatrix'].includes(operation.name)) return 'transform-map'
  if (pair === 'geometry:projection' && authoredCall) return 'projection-map'
  if (pair === 'geometry:offset' && authoredCall) return 'offset-map'
  if ((pair === 'presentation:color' || pair === 'assertion:assert') && authoredCall) return 'preserving-alias'
  if (operation.category === 'control') {
    if (operation.name === '$assign' && last.kind === 'control') return 'transparent'
    if (operation.name === '$body' && operation.parent !== null
      && last.kind === 'body' && last.ordinal === 0 && prior !== undefined
      && ['call', 'module', 'control'].includes(prior.kind)) return 'transparent'
    if ((operation.name === '$then' || operation.name === '$else')
      && last.kind === 'branch' && last.ordinal === 0
      && prior?.kind === 'control' && prior.name === 'if') return 'transparent'
    if (operation.name === '$expansion' && last.kind === 'control'
      && last.ordinal === 0 && prior?.kind === 'body' && prior.name === '$body') return 'transparent'
    if (!operation.name.startsWith('$') && last.kind === 'control'
      && ['if', 'let', 'for', 'children', 'group', 'render'].includes(operation.name)) return 'transparent'
  }
  if (operation.category === 'module' && !operation.name.startsWith('$')
    && (last.kind === 'module' || last.kind === 'call')) return 'transparent'
  if ((pair === 'boolean:union' || pair === 'boolean:intersection') && authoredCall) return 'boolean'
  if (pair === 'boolean:hull' && authoredCall) return 'hull'
  if (pair === 'boolean:difference' && authoredCall) return 'difference'
  if (pair === 'geometry:linear_extrude' && authoredCall) return 'linear-extrude'
  if (pair === 'geometry:rotate_extrude' && authoredCall) return 'rotate-extrude'
  if (allowInterruptedUnsupported) return 'terminal-unsupported'
  fail('identity', `$.core.operations[${String(operation.id)}]`, `operation pair ${pair} has no V1 production rule`)
}

function referenceResultItems(result: Record<string, any>): Record<string, any>[] {
  if (result.tag === 'empty') return []
  if (result.tag === 'single') return [result.item]
  return result.items
}

/** Independently recomputes the closed occurrence frontier and node owners. */
function validateOccurrenceProduction(
  operations: Record<string, any>[],
  occurrences: Record<string, any>[],
  nodes: Record<string, any>[],
  result: Record<string, any>,
  effects: ReferenceExecutionEffect[] = [],
  terminal: ReferenceExecutionTerminal = null,
): void {
  const terminalPrefix = terminal?.prefixFrontier ?? null
  const rowsById = new Map<string, number[]>()
  occurrences.forEach((occurrence, index) => {
    const rows = rowsById.get(occurrence.occurrenceId) ?? []
    rows.push(index)
    rowsById.set(occurrence.occurrenceId, rows)
  })
  const canonicalByRow = new Int32Array(occurrences.length)
  const groups: number[] = []
  const rowsByGroup = new Map<number, number[]>()
  for (const rows of rowsById.values()) {
    const canonical = rows[0]
    groups.push(canonical)
    rowsByGroup.set(canonical, rows)
    rows.forEach(row => { canonicalByRow[row] = canonical })
    const zero = rows.length === 1 && occurrences[rows[0]].node === null
      && occurrences[rows[0]].outputOrdinal === null && occurrences[rows[0]].sceneEntityId === null
    const output = rows.every((row, ordinal) => occurrences[row].node !== null
      && occurrences[row].outputOrdinal === ordinal && occurrences[row].sceneEntityId !== null)
    if (!zero && !output) fail('identity', `$.core.occurrences[${canonical}]`, 'logical occurrence is neither a zero frame nor dense output rows')
  }
  groups.sort((left, right) => left - right)
  const activeGroups = new Set<number>()
  let interruptedGroup: number | null = null
  if (terminal !== null) {
    let cursor: number | null = terminal.occurrence
    interruptedGroup = canonicalByRow[cursor]
    while (cursor !== null) {
      const canonical = canonicalByRow[cursor]
      activeGroups.add(canonical)
      cursor = occurrences[canonical].parent
    }
  }
  const effectByOwner = new Map<number, ReferenceExecutionEffect>()
  for (const effect of effects) {
    const owner = canonicalByRow[effect.ownerOccurrence]
    if (effectByOwner.has(owner)) {
      fail('identity', '$.core.execution.discardedEffects', 'logical occurrence owns multiple effects')
    }
    effectByOwner.set(owner, effect)
  }
  const consumedEffectOwners = new Set<number>()
  const terminalEffectRoots = new Set<number>()
  const staticChildren = new Map<number, number[]>()
  for (const operation of operations) {
    if (operation.parent === null) continue
    const list = staticChildren.get(operation.parent) ?? []
    list.push(operation.id)
    staticChildren.set(operation.parent, list)
  }
  const children = new Map<number, number[]>()
  for (const group of groups) {
    const parent = occurrences[group].parent
    if (parent === null) continue
    const parentGroup = canonicalByRow[parent]
    const list = children.get(parentGroup) ?? []
    list.push(group)
    children.set(parentGroup, list)
  }

  const owners = new Map<number, number>()
  const productions = new Map<number, ProductionGroup>()
  let expandedItems = 0
  const append = (
    target: ProductionItem[],
    source: readonly ProductionItem[],
    directChild?: number,
  ): void => {
    expandedItems += source.length
    if (expandedItems > LIMIT.snapshotValues) {
      fail('limit', '$.core.occurrences', 'expanded occurrence frontier exceeds the bounded proof budget')
    }
    for (let index = 0; index < source.length; index++) {
      const item = source[index]
      target.push(directChild === undefined ? item : { ...item, directChild })
    }
  }
  const claim = (node: number, group: number): void => {
    if (owners.has(node)) {
      fail('identity', `$.core.nodes[${node}]`, 'DAG node has more than one occurrence materializer')
    }
    owners.set(node, group)
  }
  const expectCount = (group: number, rows: number[], wanted: number): void => {
    if (rows.length !== wanted) fail('identity', `$.core.occurrences[${group}]`, `production requires ${wanted} output rows`)
  }
  const expectKind = (row: number, kinds: readonly string[]): Record<string, any> => {
    const reference = occurrences[row].node
    if (reference === null || !kinds.includes(nodes[reference].kind)) {
      fail('identity', `$.core.occurrences[${row}].node`, 'operation materializes the wrong node kind')
    }
    return nodes[reference]
  }
  const equalInputs = (left: readonly number[], right: readonly number[]): boolean => (
    left.length === right.length && left.every((item, index) => item === right[index])
  )
  const itemNodes = (items: readonly ProductionItem[]): number[] => {
    const output = new Array<number>(items.length)
    for (let index = 0; index < items.length; index++) output[index] = items[index].node
    return output
  }
  const unionNode = (
    reference: number,
    items: readonly ProductionItem[],
    group: number,
    path: string,
    schedule: ScheduleEvent[],
  ): Record<string, any> => {
    const reducer = nodes[reference]
    if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
      || !equalInputs(reducer.inputs, itemNodes(items))) {
      fail('identity', path, 'canonical reduction requires one exact ordered n-ary union')
    }
    claim(reducer.id, group)
    schedule.push({ tag: 'node', node: reducer.id })
    return reducer
  }

  // Parent rows always precede children, so reverse canonical order is an
  // iterative child-before-parent evaluation even at the 100k-row limit.
  for (let groupIndex = groups.length - 1; groupIndex >= 0; groupIndex--) {
    const group = groups[groupIndex]
    const frontier: ProductionItem[] = []
    const operation = operations[occurrences[group].operation]
    const rule = productionRule(operation, group === interruptedGroup)
    const compilerFrontierFrame = operation.category === 'control'
      && ['$body', '$then', '$else', '$expansion'].includes(operation.name)
    const buckets: ProductionItem[][] = []
    const schedule: ScheduleEvent[] = []
    const scheduleBuckets: ScheduleEvent[][] = []
    for (const child of children.get(group) ?? []) {
      const production = productions.get(child)!
      const visible = production.terminalItems
      const childEvent: ScheduleEvent = { tag: 'group', group: child }
      schedule.push(childEvent)
      if (compilerFrontierFrame) {
        const bucket: ProductionItem[] = []
        append(bucket, visible, child)
        buckets.push(bucket)
        scheduleBuckets.push([childEvent])
        append(frontier, bucket)
      } else {
        append(frontier, visible)
      }
    }
    const outputRows = rowsByGroup.get(group)!.filter(row => occurrences[row].node !== null)
    const outputItems: ProductionItem[] = []
    let terminalItems: ProductionItem[] | null = null
    const scheduleNode = (node: number): void => {
      claim(node, group)
      schedule.push({ tag: 'node', node })
    }
    const own = (row: number, node: number): void => {
      scheduleNode(node)
      outputItems.push({ node, row, identityOwner: row, group, directChild: group })
    }
    const preserve = (row: number, item: ProductionItem): void => {
      outputItems.push({ node: item.node, row, identityOwner: item.identityOwner, group, directChild: group })
    }
    const active = activeGroups.has(group)

    const differencePartitions = (): {
      buckets: ProductionItem[][]
      schedules: ScheduleEvent[][]
      activeBucket: number | null
    } => {
      const directGroups = children.get(group) ?? []
      const bodyGroups = directGroups.filter(child => {
        const childOperation = operations[occurrences[child].operation]
        const last = childOperation.structuralPath.at(-1)
        return childOperation.parent === operation.id
          && childOperation.category === 'control'
          && childOperation.name === '$body'
          && last?.kind === 'body'
          && last.name === '$body'
          && last.ordinal === 0
      })
      if (bodyGroups.length > 1
        || (bodyGroups.length === 1 && directGroups.length !== 1)
        || (bodyGroups.length === 0 && directGroups.length !== 0)) {
        fail('identity', `$.core.occurrences[${group}]`, 'difference requires at most one exact compiler-owned body child')
      }
      const bodyGroup = bodyGroups[0]
      const bodyOperation = bodyGroup === undefined ? null : occurrences[bodyGroup].operation
      const authoredOperations = bodyOperation === null
        ? []
        : (staticChildren.get(bodyOperation) ?? [])
      const bucketByOperation = new Map<number, number>()
      authoredOperations.forEach((operationId, index) => { bucketByOperation.set(operationId, index) })
      const buckets = authoredOperations.map((): ProductionItem[] => [])
      const schedules = authoredOperations.map((): ScheduleEvent[] => [])
      let activeBucket: number | null = null
      for (const runtimeGroup of bodyGroup === undefined ? [] : (children.get(bodyGroup) ?? [])) {
        const bucket = bucketByOperation.get(occurrences[runtimeGroup].operation)
        if (bucket === undefined) {
          fail('identity', `$.core.occurrences[${runtimeGroup}]`, 'difference runtime activation is not a direct authored body statement')
        }
        const production = productions.get(runtimeGroup)!
        append(buckets[bucket], production.terminalItems, runtimeGroup)
        schedules[bucket].push({ tag: 'group', group: runtimeGroup })
        if (activeGroups.has(runtimeGroup)) {
          if (activeBucket !== null) {
            fail('identity', `$.core.occurrences[${group}]`, 'difference has more than one active authored statement bucket')
          }
          activeBucket = bucket
        }
      }
      const reconstructed: ProductionItem[] = []
      for (const bucket of buckets) append(reconstructed, bucket)
      if (reconstructed.length !== frontier.length
        || reconstructed.some((item, index) => {
          const expected = frontier[index]
          return item.node !== expected.node || item.row !== expected.row
            || item.identityOwner !== expected.identityOwner
            || item.directChild !== expected.directChild
        })) {
        fail('identity', `$.core.occurrences[${group}]`, 'authored difference buckets do not exactly partition the runtime frontier')
      }
      return { buckets, schedules, activeBucket }
    }

    if (active && rule === 'difference' && outputRows.length === 0) {
      const partition = differencePartitions()
      const base = partition.buckets[0] ?? []
      const cutters: ProductionItem[] = []
      for (let bucket = 1; bucket < partition.buckets.length; bucket++) {
        append(cutters, partition.buckets[bucket])
      }
      if (group !== interruptedGroup && partition.activeBucket === null) {
        fail('identity', `$.core.occurrences[${group}]`, 'active difference does not contain the interrupted runtime bucket')
      }
      schedule.splice(0, schedule.length)
      schedule.push(...(partition.schedules[0] ?? []))
      const completedReduction = (
        items: readonly ProductionItem[],
        phase: 'base' | 'cutters',
      ): ProductionItem[] => {
        if (items.length < 2) return [...items]
        const space = nodes[items[0].node].valueType.space
        if (items.some(item => nodes[item.node].valueType.space !== space)) return [...items]
        const candidates = nodes.filter(candidate => !owners.has(candidate.id)
          && candidate.kind === 'boolean' && candidate.operation === 'union'
          && equalInputs(candidate.inputs, itemNodes(items)))
        if (candidates.length !== 1) {
          fail('identity', `$.core.occurrences[${group}]`, `completed active difference ${phase} lacks one exact union reducer`)
        }
        scheduleNode(candidates[0].id)
        return [{
          node: candidates[0].id,
          row: items[0].row,
          identityOwner: items[0].identityOwner,
          group,
          directChild: group,
        }]
      }
      // A descendant failure in the first runtime bucket interrupts base
      // evaluation before unionReduceInternal runs. Once evaluation enters a
      // cutter bucket, however, the base (and its hidden union, when needed)
      // is already complete and must be owned/scheduled by this ancestor.
      const basePhaseCompleted = group === interruptedGroup
        || partition.activeBucket! > 0
      const reducedBase = basePhaseCompleted
        ? completedReduction(base, 'base')
        : [...base]
      for (let bucket = 1; bucket < partition.schedules.length; bucket++) {
        schedule.push(...partition.schedules[bucket])
      }
      // evalNodes over all cutters must return before their partition reducer
      // is created. A descendant cutter failure therefore exposes the raw
      // completed cutter prefix; only a failure attributed to difference()
      // itself can occur after cutter reduction.
      const reducedCutters = group === interruptedGroup
        ? completedReduction(cutters, 'cutters')
        : [...cutters]
      terminalItems = [...reducedBase, ...reducedCutters]
    } else if (active && (rule === 'transform-map' || rule === 'projection-map' || rule === 'offset-map')) {
      if (outputRows.length > frontier.length) {
        fail('identity', `$.core.occurrences[${group}]`, 'partial map emits beyond its completed frontier')
      }
      const parameters = new Map<string, string>()
      outputRows.forEach((row, index) => {
        const expected = rule === 'transform-map' ? ['transform']
          : rule === 'projection-map' ? ['projection'] : ['offset']
        const produced = expectKind(row, expected)
        if (!equalInputs(nodeInputs(produced), [frontier[index].node])) {
          fail('identity', `$.core.occurrences[${row}].node`, 'partial map output skips its ordered frontier item')
        }
        const current = produced.kind === 'transform' ? stableJson(produced.matrix)
          : produced.kind === 'projection' ? String(produced.cut) : String(produced.distance)
        const parameterBucket = rule === 'transform-map' && operation.name === 'rotate'
          ? String(nodes[frontier[index].node].valueType.space)
          : 'all'
        const prior = parameters.get(parameterBucket)
        if (prior !== undefined && prior !== current) {
          fail('identity', `$.core.occurrences[${row}].node`, 'partial map outputs do not share parameters')
        }
        parameters.set(parameterBucket, current)
        scheduleNode(produced.id)
        outputItems.push({
          node: produced.id,
          row,
          identityOwner: rule === 'transform-map' ? frontier[index].identityOwner : row,
          group,
          directChild: group,
        })
      })
      terminalItems = [...outputItems]
      append(terminalItems, frontier.slice(outputRows.length))
    } else if (rule === 'terminal-unsupported') {
      if (!active || group !== interruptedGroup) {
        fail('identity', `$.core.occurrences[${group}]`, 'unsupported production is not the exact interrupted activation')
      }
      expectCount(group, outputRows, 0)
      terminalItems = [...frontier]
    } else if (active && outputRows.length === 0) {
      // An operation can fail after completing children, or an ancestor can be
      // suspended while its active descendant fails. Completed child groups
      // remain strict; only this active-chain group's absent result is open.
      terminalItems = [...frontier]
    } else if (rule === 'transparent') {
      expectCount(group, outputRows, 0)
    } else if (rule === 'primitive') {
      if (frontier.length !== 0) fail('identity', `$.core.occurrences[${group}]`, 'primitive frontier is not empty')
      expectCount(group, outputRows, 1)
      const kinds: Readonly<Record<string, readonly string[]>> = {
        cube: ['box'], sphere: ['sphere-analytic', 'sphere-polygonal'],
        cylinder: ['cylinder-analytic', 'cylinder-polygonal'], polyhedron: ['polyhedron'],
        square: ['rectangle'], circle: ['circle-analytic', 'circle-polygonal'], polygon: ['polygon'],
      }
      const produced = expectKind(outputRows[0], kinds[operation.name])
      own(outputRows[0], produced.id)
    } else if (rule === 'transform-map' || rule === 'projection-map' || rule === 'offset-map') {
      expectCount(group, outputRows, frontier.length)
      const parameters = new Map<string, string>()
      outputRows.forEach((row, index) => {
        const expected = rule === 'transform-map' ? ['transform']
          : rule === 'projection-map' ? ['projection'] : ['offset']
        const produced = expectKind(row, expected)
        if (!equalInputs(nodeInputs(produced), [frontier[index].node])) {
          fail('identity', `$.core.occurrences[${row}].node`, 'map output does not consume its ordered frontier item')
        }
        const current = produced.kind === 'transform' ? stableJson(produced.matrix)
          : produced.kind === 'projection' ? String(produced.cut) : String(produced.distance)
        const parameterBucket = rule === 'transform-map' && operation.name === 'rotate'
          ? String(nodes[frontier[index].node].valueType.space)
          : 'all'
        const prior = parameters.get(parameterBucket)
        if (prior !== undefined && prior !== current) {
          fail('identity', `$.core.occurrences[${row}].node`, 'map outputs do not share parameters')
        }
        parameters.set(parameterBucket, current)
        scheduleNode(produced.id)
        if (rule === 'transform-map') {
          outputItems.push({
            node: produced.id,
            row,
            identityOwner: frontier[index].identityOwner,
            group,
            directChild: group,
          })
        }
        else outputItems.push({ node: produced.id, row, identityOwner: row, group, directChild: group })
      })
    } else if (rule === 'preserving-alias') {
      expectCount(group, outputRows, frontier.length)
      outputRows.forEach((row, index) => {
        if (occurrences[row].node !== frontier[index].node) {
          fail('identity', `$.core.occurrences[${row}].node`, 'alias changes its frontier node')
        }
        preserve(row, frontier[index])
      })
    } else if (rule === 'boolean' || rule === 'hull') {
      if (frontier.length === 0) {
        expectCount(group, outputRows, 0)
      } else if (frontier.length === 1) {
        expectCount(group, outputRows, 1)
        if (occurrences[outputRows[0]].node !== frontier[0].node) {
          fail('identity', `$.core.occurrences[${outputRows[0]}].node`, 'one-item reduction changes its frontier node')
        }
        if (rule === 'hull') outputItems.push({ node: frontier[0].node, row: outputRows[0], identityOwner: outputRows[0], group, directChild: group })
        else preserve(outputRows[0], frontier[0])
      } else {
        expectCount(group, outputRows, 1)
        const produced = expectKind(outputRows[0], rule === 'hull' ? ['hull'] : ['boolean'])
        if (produced.kind === 'boolean') {
          if (produced.operation !== operation.name) fail('identity', `$.core.nodes[${produced.id}].operation`, 'Boolean operation disagrees with static rule')
        }
        if (!equalInputs(nodeInputs(produced), itemNodes(frontier))) {
          fail('identity', `$.core.nodes[${produced.id}]`, 'reduction inputs do not equal the ordered frontier')
        }
        own(outputRows[0], produced.id)
      }
    } else if (rule === 'difference') {
      const partition = differencePartitions()
      const base = partition.buckets[0] ?? []
      const cutters: ProductionItem[] = []
      for (let bucket = 1; bucket < partition.buckets.length; bucket++) {
        append(cutters, partition.buckets[bucket])
      }
      schedule.splice(0, schedule.length)
      schedule.push(...(partition.schedules[0] ?? []))
      const appendCutterSchedule = (): void => {
        for (let bucket = 1; bucket < partition.schedules.length; bucket++) {
          schedule.push(...partition.schedules[bucket])
        }
      }
      const effect = effectByOwner.get(group)
      if (effect !== undefined) {
        expectCount(group, outputRows, 0)
        if (base.length > 0) {
          fail('identity', '$.core.execution.discardedEffects', 'discarded cutters require an empty first difference activation')
        }
        if (cutters.length === 0) {
          fail('identity', '$.core.execution.discardedEffects', 'empty difference frontier cannot yield an effect')
        }
        appendCutterSchedule()
        consumedEffectOwners.add(group)
        if (cutters.length === 1) {
          if (effect.root !== cutters[0].node) {
            fail('identity', '$.core.execution.discardedEffects', 'single discarded cutter root is not exact')
          }
        } else {
          unionNode(effect.root, cutters, group, '$.core.execution.discardedEffects', schedule)
        }
      } else if (base.length === 0) {
        expectCount(group, outputRows, 0)
        appendCutterSchedule()
        if (cutters.length > 0) {
          if (terminalPrefix === null) {
            fail('identity', `$.core.occurrences[${group}]`, 'empty-base difference omits its eager cutter effect')
          }
          const root = cutters.length === 1
            ? cutters[0].node
            : terminalPrefix.find(entry => {
              const candidate = nodes[entry.root]
              return candidate?.kind === 'boolean' && candidate.operation === 'union'
                && equalInputs(candidate.inputs, itemNodes(cutters))
            })?.root
          if (root === undefined || !terminalPrefix.some(entry => entry.root === root)) {
            fail('identity', '$.core.execution.terminal.prefixFrontier', 'terminal omits an eager difference cutter root')
          }
          if (cutters.length > 1) {
            unionNode(root, cutters, group, '$.core.execution.terminal.prefixFrontier', schedule)
          }
          consumedEffectOwners.add(group)
          terminalEffectRoots.add(root)
        }
      } else if (cutters.length === 0) {
        expectCount(group, outputRows, 1)
        const row = outputRows[0]
        if (base.length === 1) {
          if (occurrences[row].node !== base[0].node) {
            fail('identity', `$.core.occurrences[${row}].node`, 'difference without cutters must return its sole base item')
          }
          preserve(row, base[0])
        } else {
          const reference = occurrences[row].node
          if (reference === null) fail('identity', `$.core.occurrences[${row}].node`, 'difference base reduction is absent')
          unionNode(reference, base, group, `$.core.occurrences[${row}].node`, schedule)
          outputItems.push({ node: reference, row, identityOwner: row, group, directChild: group })
        }
        appendCutterSchedule()
      } else {
        expectCount(group, outputRows, 1)
        const row = outputRows[0]
        const produced = expectKind(row, ['boolean'])
        if (produced.operation !== 'difference' || produced.inputs.length !== 2) {
          fail('identity', `$.core.nodes[${produced.id}]`, 'difference must own one binary partition root')
        }
        const reducedBase = base.length === 1
          ? base[0].node
          : unionNode(produced.inputs[0], base, group, `$.core.nodes[${produced.id}].inputs[0]`, schedule).id
        appendCutterSchedule()
        const reducedCutters = cutters.length === 1
          ? cutters[0].node
          : unionNode(produced.inputs[1], cutters, group, `$.core.nodes[${produced.id}].inputs[1]`, schedule).id
        if (!equalInputs(produced.inputs, [reducedBase, reducedCutters])) {
          fail('identity', `$.core.nodes[${produced.id}].inputs`, 'difference swaps or escapes its base/cutter partition')
        }
        own(row, produced.id)
      }
    } else if (frontier.length === 0) {
      expectCount(group, outputRows, 0)
    } else {
      expectCount(group, outputRows, 1)
      const produced = expectKind(outputRows[0], rule === 'linear-extrude'
        ? ['linear-extrude'] : ['rotate-extrude-analytic', 'rotate-extrude-polygonal'])
      const directInput = nodeInputs(produced)[0]
      if (frontier.length === 1) {
        if (directInput !== frontier[0].node) fail('identity', `$.core.nodes[${produced.id}].input`, 'extrusion skips its sole profile')
      } else {
        const reducer = nodes[directInput]
        if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
          || !equalInputs(reducer.inputs, itemNodes(frontier))) {
          fail('identity', `$.core.nodes[${produced.id}].input`, 'extrusion lacks its canonical union reducer')
        }
        scheduleNode(reducer.id)
      }
      own(outputRows[0], produced.id)
    }
    if (terminalItems === null) {
      terminalItems = outputItems.length > 0
        ? [...outputItems]
        : rule === 'transparent' ? [...frontier] : []
    }
    productions.set(group, {
      outputRows,
      outputItems,
      frontier,
      terminalItems,
      buckets,
      schedule,
      scheduleBuckets,
      transparent: rule === 'transparent',
    })
  }

  if ([...effectByOwner.keys()].some(owner => !consumedEffectOwners.has(owner))) {
    fail('identity', '$.core.execution.discardedEffects', 'effect owner is not an exact empty-result difference')
  }
  if (owners.size !== nodes.length) {
    const unowned = nodes.find(node => !owners.has(node.id))
    fail('identity', unowned ? `$.core.nodes[${unowned.id}]` : '$.core.nodes', 'reachable node lacks exactly one production owner')
  }
  const scheduleStack: ScheduleEvent[] = []
  for (let index = groups.length - 1; index >= 0; index--) {
    const group = groups[index]
    if (occurrences[group].parent === null) scheduleStack.push({ tag: 'group', group })
  }
  const derivedSchedule: number[] = []
  let scheduleEvents = 0
  while (scheduleStack.length > 0) {
    if (++scheduleEvents > LIMIT.occurrences + LIMIT.nodes * 2) {
      fail('limit', '$.core.execution.evaluationOrder', 'schedule replay exceeds its bounded event budget')
    }
    const event = scheduleStack.pop()!
    if (event.tag === 'node') {
      derivedSchedule.push(event.node)
      if (derivedSchedule.length > nodes.length) {
        fail('order', '$.core.execution.evaluationOrder', 'replayed schedule creates a node more than once')
      }
      continue
    }
    const childSchedule = productions.get(event.group)?.schedule
    if (childSchedule === undefined) {
      fail('reference', '$.core.execution.evaluationOrder', 'schedule references an absent production group')
    }
    for (let index = childSchedule.length - 1; index >= 0; index--) {
      scheduleStack.push(childSchedule[index])
    }
  }
  if (derivedSchedule.length !== nodes.length
    || derivedSchedule.some((node, index) => node !== index)) {
    fail('order', '$.core.execution.evaluationOrder', 'node IDs do not equal independently replayed kernel creation order')
  }
  const reachable = new Set<number>()
  const pending: number[] = []
  const programFrontier: ProductionItem[] = []
  for (const group of groups) {
    if (occurrences[group].parent !== null) continue
    const production = productions.get(group)!
    append(programFrontier, production.terminalItems)
  }
  const resultItems = referenceResultItems(result)
  if (terminalPrefix === null) {
    if (resultItems.length !== programFrontier.length) {
      fail('identity', '$.core.result', 'result does not exactly equal the synthetic program frontier')
    }
    for (let index = 0; index < resultItems.length; index++) {
      const output = resultItems[index]
      const slot = programFrontier[index]
      if (slot.row !== output.producerOccurrence || slot.node !== output.node) {
        fail('identity', '$.core.result', 'root producer is not an emitted production slot')
      }
      if (slot.identityOwner !== output.identityOccurrence) {
        fail('identity', '$.core.result', 'root identity owner is not recursively derived')
      }
      pending.push(slot.group)
    }
  } else {
    if (resultItems.length !== 0) fail('identity', '$.core.result', 'terminal result is not bottom')
    const derivedRoots = [...new Set([
      ...programFrontier.map(item => item.node),
      ...terminalEffectRoots,
    ])].sort((left, right) => left - right)
    if (derivedRoots.length !== terminalPrefix.length
      || derivedRoots.some((root, index) => root !== terminalPrefix[index].root)) {
      fail('identity', '$.core.execution.terminal.prefixFrontier', 'terminal prefix does not equal the completed production frontier')
    }
    for (const slot of programFrontier) pending.push(slot.group)
  }
  for (const owner of consumedEffectOwners) pending.push(owner)
  while (pending.length > 0) {
    const group = pending.pop()!
    if (reachable.has(group)) continue
    reachable.add(group)
    for (const item of productions.get(group)!.frontier) pending.push(item.group)
  }
  for (const group of groups) {
    if (productions.get(group)!.outputRows.length > 0 && !reachable.has(group)) {
      fail('identity', `$.core.occurrences[${group}]`, 'node-bearing production group is detached from all roots')
    }
  }
}

export function referenceSemanticCapabilityClosureV1(core: Record<string, any>): string[] {
  const result = record(core.result, '$.core.result')
  const capabilities = new Set<string>(core.declaredCapabilities as string[])
  capabilities.add('semantic.program-v1')
  capabilities.add('semantic.operation-graph')
  capabilities.add('semantic.identity-evidence')
  capabilities.add(`semantic.result.${String(result.tag)}`)
  if ((core.occurrences as unknown[]).length > 0) capabilities.add('semantic.occurrences')
  if ((core.diagnosticTemplates as unknown[]).length > 0) capabilities.add('diagnostics.deterministic')
  for (const node of core.nodes as Record<string, any>[]) {
    const capability = (CAPABILITY_BY_KIND as Readonly<Record<string, string>>)[node.kind]
    if (capability === undefined) fail('field', '$.core.nodes', 'unknown node capability tag')
    capabilities.add(capability)
    capabilities.add(`geometry.kind.${String(node.valueType.geometryKind)}`)
    capabilities.add(`geometry.space.${String(node.valueType.space)}`)
    capabilities.add(`representation.${String(node.valueType.representation)}`)
    capabilities.add(`evidence.${String(node.valueType.evidence.tag)}`)
    if (node.valueType.evidence.tag === 'certified-approximation') {
      capabilities.add(`evidence.profile.${String(node.valueType.evidence.certificateProfile)}`)
      capabilities.add(`evidence.policy.sha256.${String(node.valueType.evidence.certificatePolicyHash)}`)
    }
    capabilities.add('geometry.value')
    if (node.kind === 'boolean') {
      capabilities.add('operation.boolean')
      capabilities.add(`operation.boolean.${String(node.operation)}`)
    }
    for (const inputIndex of nodeInputs(node)) {
      const inputType = core.nodes[inputIndex].valueType
      if (inputType.geometryKind !== node.valueType.geometryKind
        || inputType.space !== node.valueType.space) {
        capabilities.add(
          `geometry.transition.${String(inputType.geometryKind)}.${String(inputType.space)}`
          + `.to.${String(node.valueType.geometryKind)}.${String(node.valueType.space)}`,
        )
      }
      if (inputType.representation !== node.valueType.representation) {
        capabilities.add(`representation.transition.${String(inputType.representation)}.to.${String(node.valueType.representation)}`)
      }
      if (inputType.evidence.tag !== node.valueType.evidence.tag) {
        capabilities.add(`evidence.transition.${String(inputType.evidence.tag)}.to.${String(node.valueType.evidence.tag)}`)
      }
    }
  }
  return [...capabilities].sort(utf8Compare)
}

function validateCore(value: unknown): Record<string, any> {
  const core = exact(value, KEY.core, '$.core')
  if (core.schema !== 'semantic-program-core') fail('schema', '$.core.schema', 'wrong core schema')
  const version = exact(core.schemaVersion, KEY.version, '$.core.schemaVersion')
  if (version.major !== 1 || version.minor !== 2) fail('version', '$.core.schemaVersion', 'only 1.2 is supported')
  const required = sortedIdentifiers(core.requiredFeatures, '$.core.requiredFeatures', 64)
  if (required.length !== 1 || required[0] !== 'semantic.execution-v2') {
    fail('version', '$.core.requiredFeatures', 'v1.2 requires exactly semantic.execution-v2')
  }
  if (core.identityVersion !== 'semantic-program-core-v1') fail('version', '$.core.identityVersion', 'unknown identity version')
  const language = exact(core.language, KEY.language, '$.core.language')
  if (!includes(TAG.language, language.contract)) fail('field', '$.core.language.contract', 'unknown language contract')
  const revision = language.contract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0'
  if (language.semanticsRevision !== revision) fail('version', '$.core.language.semanticsRevision', 'wrong semantic revision')
  if (language.capabilityGraphVersion !== 'semantic-capabilities-v1') fail('version', '$.core.language.capabilityGraphVersion', 'unknown capability graph')
  const units = exact(core.units, KEY.units, '$.core.units')
  if (units.length !== 'millimeter' || units.angle !== 'degree' || units.handedness !== 'right'
    || units.upAxis !== 'z' || units.matrixLayout !== 'column-major'
    || units.composition !== 'parent-times-local') {
    fail('type', '$.core.units', 'units/frame convention is fixed')
  }

  const operations = validateOperations(core.operations)
  const nodes = validateNodes(core.nodes, language.contract)
  const occurrences = validateOccurrences(core.occurrences, operations, nodes)
  const templates = validateDiagnosticTemplates(core.diagnosticTemplates, operations)
  const outputRoots = validateResult(core.result, nodes, occurrences)
  const execution = validateExecution(
    core.execution,
    nodes,
    operations,
    occurrences,
    outputRoots,
    templates,
    language.contract,
  )
  validateOccurrenceProduction(
    operations,
    occurrences,
    nodes,
    core.result,
    execution.discardedEffects,
    execution.terminal,
  )

  sortedIdentifiers(core.declaredCapabilities, '$.core.declaredCapabilities', LIMIT.declaredCapabilities)
  const closure = sortedIdentifiers(core.capabilityClosure, '$.core.capabilityClosure', LIMIT.capabilityClosure)
  const expectedClosure = referenceSemanticCapabilityClosureV1(core)
  if (closure.length !== expectedClosure.length || closure.some((item, index) => item !== expectedClosure[index])) {
    fail('closure', '$.core.capabilityClosure', 'not the exact authored and inferred closure')
  }
  return core
}

function validateSpan(value: unknown, path: string, sourceLength: number, emptyAllowed: boolean): void {
  const span = exact(value, KEY.span, path)
  const start = integer(span.start, `${path}.start`, 0, sourceLength)
  const end = integer(span.end, `${path}.end`, 0, sourceLength)
  if (end < start || (!emptyAllowed && end === start)) fail('source', path, 'invalid half-open UTF-16 span')
}

function validateReferenceEnvelopeSnapshot(
  snapshot: unknown,
  exactSource?: string,
): ReferenceSemanticProgramV1 {
  const envelope = exact(snapshot, KEY.envelope, '$')
  if (envelope.schema !== 'semantic-program-envelope') fail('schema', '$.schema', 'wrong envelope schema')
  const version = exact(envelope.schemaVersion, KEY.version, '$.schemaVersion')
  if (version.major !== 1 || version.minor !== 2) fail('version', '$.schemaVersion', 'only 1.2 is supported')
  const source = exact(envelope.source, KEY.source, '$.source')
  if (typeof source.sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(source.sha256)) fail('field', '$.source.sha256', 'not lowercase SHA-256')
  integer(source.utf8ByteLength, '$.source.utf8ByteLength', 0, LIMIT.sourceUtf8Bytes)
  const sourceLength = integer(source.utf16CodeUnitLength, '$.source.utf16CodeUnitLength', 0, LIMIT.sourceUtf16Units)
  const core = validateCore(envelope.core)

  const provenance = array(envelope.provenance, '$.provenance', LIMIT.operations)
  if (provenance.length !== core.operations.length) fail('source', '$.provenance', 'one record required per operation')
  provenance.forEach((item, index) => {
    const path = `$.provenance[${index}]`
    const entry = exact(item, KEY.provenance, path)
    if (entry.operation !== index) fail('order', `${path}.operation`, 'records must follow operation order')
    validateSpan(entry.span, `${path}.span`, sourceLength, false)
    text(entry.label, `${path}.label`, 512)
  })

  const intents = array(envelope.tessellationIntents, '$.tessellationIntents', LIMIT.tessellationIntents)
  if (core.language.contract === 'legacy/current' && intents.length !== 0) {
    fail('type', '$.tessellationIntents', 'legacy contract already materializes tessellation')
  }
  let priorOccurrence = -1
  intents
    .forEach((item, index) => {
      const path = `$.tessellationIntents[${index}]`
      const intent = exact(item, KEY.tessellation, path)
      const occurrence = integer(intent.occurrence, `${path}.occurrence`, 0, core.occurrences.length - 1)
      if (core.occurrences[occurrence].node === null) fail('reference', `${path}.occurrence`, 'intent needs producing occurrence')
      if (occurrence <= priorOccurrence) fail('order', `${path}.occurrence`, 'intents must be unique and ordered')
      priorOccurrence = occurrence
      if (intent.chordTolerance !== null) positive(intent.chordTolerance, `${path}.chordTolerance`)
      if (intent.angularToleranceDegrees !== null) positive(intent.angularToleranceDegrees, `${path}.angularToleranceDegrees`)
      const minimum = intent.minSegments === null ? null : integer(intent.minSegments, `${path}.minSegments`, 3, 1_000_000)
      const maximum = intent.maxSegments === null ? null : integer(intent.maxSegments, `${path}.maxSegments`, 3, 1_000_000)
      if (minimum !== null && maximum !== null && maximum < minimum) fail('number', path, 'maxSegments below minSegments')
    })

  const diagnostics = array(envelope.diagnostics, '$.diagnostics', LIMIT.diagnostics)
  if (diagnostics.length !== core.diagnosticTemplates.length) fail('source', '$.diagnostics', 'one presentation required per template')
  diagnostics.forEach((item, index) => {
    const path = `$.diagnostics[${index}]`
    const diagnostic = exact(item, KEY.diagnostic, path)
    if (diagnostic.template !== index) fail('order', `${path}.template`, 'presentations must follow template order')
    text(diagnostic.message, `${path}.message`, 2_048)
    if (diagnostic.span !== null) validateSpan(diagnostic.span, `${path}.span`, sourceLength, true)
  })

  if (exactSource !== undefined) {
    const actual = referenceSemanticSourceDescriptorV1(exactSource)
    if (actual.sha256 !== source.sha256
      || actual.utf8ByteLength !== source.utf8ByteLength
      || actual.utf16CodeUnitLength !== source.utf16CodeUnitLength) {
      fail('source', '$.source', 'source bytes do not match attestation')
    }
    const route = referenceRoutingHeader(exactSource)
    if (route.contract !== core.language.contract) {
      fail('source', '$.core.language.contract', 'routing language disagrees with core')
    }
    if (route.capabilities.length !== core.declaredCapabilities.length
      || route.capabilities.some((capability, index) => capability !== core.declaredCapabilities[index])) {
      fail('source', '$.core.declaredCapabilities', 'routing requirements disagree with core')
    }
    const scalarBoundary = (offset: number, path: string): void => {
      if (offset <= 0 || offset >= exactSource.length) return
      const high = exactSource.charCodeAt(offset - 1)
      const low = exactSource.charCodeAt(offset)
      if (high >= 0xd800 && high <= 0xdbff && low >= 0xdc00 && low <= 0xdfff) {
        fail('source', path, 'span endpoint splits a surrogate pair')
      }
    }
    envelope.provenance.forEach((entry: Record<string, any>, index: number) => {
      scalarBoundary(entry.span.start, `$.provenance[${index}].span.start`)
      scalarBoundary(entry.span.end, `$.provenance[${index}].span.end`)
    })
    envelope.diagnostics.forEach((entry: Record<string, any>, index: number) => {
      if (entry.span === null) return
      scalarBoundary(entry.span.start, `$.diagnostics[${index}].span.start`)
      scalarBoundary(entry.span.end, `$.diagnostics[${index}].span.end`)
    })
  }
  return envelope
}

function normalizeReferenceSemanticProgramV1(
  input: unknown,
  exactSource?: string,
): ReferenceSemanticProgramV1 {
  return validateReferenceEnvelopeSnapshot(snapshotJsonTree(input), exactSource)
}

function normalizeReferenceSemanticProgramCoreV1(
  input: unknown,
  exactSource?: string,
): Record<string, any> {
  const snapshot = snapshotJsonTree(input)
  const candidate = record(snapshot, '$')
  if (candidate.schema === 'semantic-program-core') return validateCore(candidate)
  return validateReferenceEnvelopeSnapshot(snapshot, exactSource).core
}

export function referenceValidateSemanticProgramV1(
  input: unknown,
  exactSource?: string,
): asserts input is ReferenceSemanticProgramV1 {
  normalizeReferenceSemanticProgramV1(input, exactSource)
}

class ChunkWriter {
  private readonly chunks: Uint8Array[] = []
  private byteLength = 0

  private append(bytes: Uint8Array): void {
    if (this.byteLength + bytes.length > LIMIT.binaryBytes) fail('limit', '$binary', 'canonical frame exceeds 64 MiB')
    this.chunks.push(bytes)
    this.byteLength += bytes.length
  }

  byte(value: number): void { this.append(Uint8Array.of(value & 0xff)) }
  u16(value: number): void {
    if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff) fail('limit', '$binary', 'u16 overflow')
    this.append(Uint8Array.of((value >>> 8) & 0xff, value & 0xff))
  }
  u32(value: number): void {
    if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff_ffff) fail('limit', '$binary', 'u32 overflow')
    this.append(u32be(value))
  }
  f64(value: number): void {
    number(value, '$binary.number')
    const bytes = new Uint8Array(8)
    new DataView(bytes.buffer).setFloat64(0, value, true)
    this.append(bytes)
  }
  raw(value: Uint8Array): void { this.append(value) }
  finish(): Uint8Array {
    const output = new Uint8Array(this.byteLength)
    let offset = 0
    for (const chunk of this.chunks) {
      output.set(chunk, offset)
      offset += chunk.length
    }
    return output
  }
}

interface EncodingBudget { values: number; stringBytes: number }

function addStringBytes(length: number, budget: EncodingBudget): void {
  if (length > LIMIT.stringCodeUnits * 4) fail('limit', '$binary', 'individual string exceeds byte limit')
  budget.stringBytes += length
  if (budget.stringBytes > BINARY_STRING_BYTES) fail('limit', '$binary', 'aggregate string bytes exceed 16 MiB')
}

function encodeCanonicalValue(value: unknown, writer: ChunkWriter, budget: EncodingBudget, depth = 0): void {
  if (++budget.values > LIMIT.snapshotValues) fail('limit', '$binary', 'value budget exceeded')
  if (depth > LIMIT.snapshotDepth) fail('limit', '$binary', 'depth exceeded')
  if (value === null) { writer.byte(WIRE_TAG.null); return }
  if (value === false) { writer.byte(WIRE_TAG.false); return }
  if (value === true) { writer.byte(WIRE_TAG.true); return }
  if (typeof value === 'number') {
    writer.byte(WIRE_TAG.number)
    writer.f64(value)
    return
  }
  if (typeof value === 'string') {
    const bytes = UTF8.encode(value)
    addStringBytes(bytes.length, budget)
    writer.byte(WIRE_TAG.string)
    writer.u32(bytes.length)
    writer.raw(bytes)
    return
  }
  if (Array.isArray(value)) {
    writer.byte(WIRE_TAG.array)
    writer.u32(value.length)
    value.forEach(item => encodeCanonicalValue(item, writer, budget, depth + 1))
    return
  }
  if (isRecord(value)) {
    const entries = Object.entries(value)
      .map(([key, item]) => ({ key: UTF8.encode(key), item }))
      .sort((left, right) => byteCompare(left.key, right.key))
    writer.byte(WIRE_TAG.object)
    writer.u32(entries.length)
    for (const entry of entries) {
      addStringBytes(entry.key.length, budget)
      writer.u32(entry.key.length)
      writer.raw(entry.key)
      encodeCanonicalValue(entry.item, writer, budget, depth + 1)
    }
    return
  }
  fail('schema', '$binary', 'only JSON values are encodable')
}

function frame(magic: readonly number[], payload: unknown): Uint8Array {
  const bodyWriter = new ChunkWriter()
  encodeCanonicalValue(payload, bodyWriter, { values: 0, stringBytes: 0 })
  const body = bodyWriter.finish()
  const writer = new ChunkWriter()
  writer.raw(Uint8Array.from(magic))
  writer.u16(1)
  writer.u16(2)
  writer.u32(body.length)
  writer.raw(body)
  return writer.finish()
}

export function referenceEncodeSemanticProgramV1(
  input: unknown,
  exactSource?: string,
): Uint8Array {
  const snapshot = normalizeReferenceSemanticProgramV1(input, exactSource)
  return frame([0x53, 0x50, 0x45, 0x31], snapshot)
}

export function referenceEncodeSemanticProgramCoreV1(
  input: unknown,
  exactSource?: string,
): Uint8Array {
  const core = normalizeReferenceSemanticProgramCoreV1(input, exactSource)
  return frame([0x53, 0x50, 0x43, 0x31], core)
}

export function referenceSemanticProgramHashV1(
  input: unknown,
  exactSource?: string,
): string {
  const core = referenceEncodeSemanticProgramCoreV1(input, exactSource)
  return sha256(lengthPrefixed(UTF8.encode('semantic-program-core-v1'), core))
}

export const REFERENCE_SEMANTIC_SOURCE_V1 = [
  '// π🙂 independent semantic fixture',
  'module scene() {',
  '  translate([4, 5, 6])',
  '    union() {',
  '      cube([1, 2, 3]);',
  '      sphere(2, $fn=32);',
  '    }',
  '}',
].join('\n')

function tokenSpan(source: string, token: string): { start: number; end: number } {
  const start = source.indexOf(token)
  if (start < 0) throw new Error(`fixture token ${token} is absent`)
  return { start, end: start + token.length }
}

export function makeReferenceSemanticProgramV1(): ReferenceSemanticProgramV1 {
  const source = REFERENCE_SEMANTIC_SOURCE_V1
  const paths = [
    [{ kind: 'module', name: 'scene', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'translate', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'translate', ordinal: 0 }, { kind: 'call', name: 'union', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'translate', ordinal: 0 }, { kind: 'call', name: 'union', ordinal: 0 }, { kind: 'call', name: 'cube', ordinal: 0 }],
    [{ kind: 'module', name: 'scene', ordinal: 0 }, { kind: 'call', name: 'translate', ordinal: 0 }, { kind: 'call', name: 'union', ordinal: 0 }, { kind: 'call', name: 'sphere', ordinal: 0 }],
  ]
  const operationIds = paths.map(referenceSemanticOperationIdV1)
  const operations = [
    { id: 0, operationId: operationIds[0], parent: null, childOrdinal: 0, name: 'scene', category: 'module', structuralPath: paths[0], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 1, operationId: operationIds[1], parent: 0, childOrdinal: 0, name: 'translate', category: 'transform', structuralPath: paths[1], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 2, operationId: operationIds[2], parent: 1, childOrdinal: 0, name: 'union', category: 'boolean', structuralPath: paths[2], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 3, operationId: operationIds[3], parent: 2, childOrdinal: 0, name: 'cube', category: 'geometry', structuralPath: paths[3], identityEvidence: 'structural-unique', ambiguityGroup: null },
    { id: 4, operationId: operationIds[4], parent: 2, childOrdinal: 0, name: 'sphere', category: 'geometry', structuralPath: paths[4], identityEvidence: 'structural-unique', ambiguityGroup: null },
  ]
  const slots = [[], [], [], [], [{
    name: '$fn',
    value: { tag: 'number', value: 32 },
    duplicateOrdinal: 0,
  }]]
  const parents: Array<number | null> = [null, 0, 1, 2, 2]
  const occurrenceIds: string[] = []
  for (let index = 0; index < operations.length; index++) {
    const parent = parents[index]
    occurrenceIds.push(referenceSemanticOccurrenceIdV1(
      parent === null ? null : occurrenceIds[parent],
      parent === null ? null : occurrenceIds[parent],
      operationIds[index],
      slots[index],
    ))
  }
  const scene = (index: number): string => referenceSemanticSceneEntityIdV1(occurrenceIds[index], 0)
  const occurrences = [
    { id: 0, occurrenceId: occurrenceIds[0], operation: 0, parent: null, staticParent: null, dynamicSlots: slots[0], node: null, outputOrdinal: null, sceneEntityId: null },
    { id: 1, occurrenceId: occurrenceIds[1], operation: 1, parent: 0, staticParent: 0, dynamicSlots: slots[1], node: 3, outputOrdinal: 0, sceneEntityId: scene(1) },
    { id: 2, occurrenceId: occurrenceIds[2], operation: 2, parent: 1, staticParent: 1, dynamicSlots: slots[2], node: 2, outputOrdinal: 0, sceneEntityId: scene(2) },
    { id: 3, occurrenceId: occurrenceIds[3], operation: 3, parent: 2, staticParent: 2, dynamicSlots: slots[3], node: 0, outputOrdinal: 0, sceneEntityId: scene(3) },
    { id: 4, occurrenceId: occurrenceIds[4], operation: 4, parent: 2, staticParent: 2, dynamicSlots: slots[4], node: 1, outputOrdinal: 0, sceneEntityId: scene(4) },
  ]
  const program: ReferenceSemanticProgramV1 = {
    schema: 'semantic-program-envelope',
    schemaVersion: { major: 1, minor: 2 },
    source: referenceSemanticSourceDescriptorV1(source),
    core: {
      schema: 'semantic-program-core',
      schemaVersion: { major: 1, minor: 2 },
      requiredFeatures: ['semantic.execution-v2'],
      identityVersion: 'semantic-program-core-v1',
      language: {
        contract: 'legacy/current',
        semanticsRevision: '1.0.0',
        capabilityGraphVersion: 'semantic-capabilities-v1',
      },
      units: {
        length: 'millimeter',
        angle: 'degree',
        handedness: 'right',
        upAxis: 'z',
        matrixLayout: 'column-major',
        composition: 'parent-times-local',
      },
      operations,
      occurrences,
      nodes: [
        { id: 0, kind: 'box', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, size: [1, 2, 3], center: false },
        { id: 1, kind: 'sphere-polygonal', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, radius: 2, radialSegments: 32 },
        { id: 2, kind: 'boolean', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, operation: 'union', inputs: [0, 1] },
        { id: 3, kind: 'transform', valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } }, input: 2, matrix: [
          1, 0, 0, 0,
          0, 1, 0, 0,
          0, 0, 1, 0,
          4, 5, 6, 1,
        ] },
      ],
      execution: {
        version: 'semantic-execution-v2',
        evaluationOrder: [0, 1, 2, 3],
        discardedEffects: [],
        terminal: null,
      },
      result: {
        tag: 'single',
        item: { node: 3, producerOccurrence: 1, identityOccurrence: 2, color: [0.25, 0.5, 0.75, 1] },
      },
      declaredCapabilities: [],
      capabilityClosure: [],
      diagnosticTemplates: [{
        id: 0,
        code: 'SEMANTIC_NOTE',
        severity: 'info',
        operation: 2,
        arguments: [{ name: 'inputs', value: { tag: 'number', value: 2 } }],
      }],
    },
    provenance: [
      { operation: 0, span: tokenSpan(source, 'module scene'), label: 'scene()' },
      { operation: 1, span: tokenSpan(source, 'translate'), label: 'translate()' },
      { operation: 2, span: tokenSpan(source, 'union'), label: 'union()' },
      { operation: 3, span: tokenSpan(source, 'cube'), label: 'cube()' },
      { operation: 4, span: tokenSpan(source, 'sphere'), label: 'sphere()' },
    ],
    tessellationIntents: [],
    diagnostics: [{ template: 0, message: 'independent reference diagnostic', span: null }],
  }
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return program
}

export function makeReferenceLinearChainProgramV1(length: number): ReferenceSemanticProgramV1 {
  integer(length, '$fixture.length', 1, LIMIT.nodes)
  const source = 'chain();'
  const nodes: Record<string, any>[] = [{
    id: 0,
    kind: 'box',
    valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } },
    size: [1, 1, 1],
    center: false,
  }]
  const identityMatrix = [
    1, 0, 0, 0,
    0, 1, 0, 0,
    0, 0, 1, 0,
    0, 0, 0, 1,
  ]
  for (let index = 1; index < length; index++) {
    nodes.push({
      id: index,
      kind: 'transform',
      valueType: { geometryKind: 'solid-set', space: 'd3', representation: 'mesh', evidence: { tag: 'representation-preserving' } },
      input: index - 1,
      matrix: [...identityMatrix],
    })
  }
  const operations: Record<string, any>[] = []
  const appendOperation = (
    parent: number | null,
    kind: string,
    name: string,
    category: string,
    childOrdinal = 0,
    identityEvidence = 'structural-unique',
    ambiguityGroup: string | null = null,
  ): number => {
    const structuralPath = [
      ...(parent === null ? [] : operations[parent].structuralPath),
      { kind, name, ordinal: childOrdinal },
    ]
    const id = operations.length
    operations.push({
      id,
      operationId: referenceSemanticOperationIdV1(structuralPath),
      parent,
      childOrdinal,
      name,
      category,
      structuralPath,
      identityEvidence,
      ambiguityGroup,
    })
    return id
  }
  const occurrences: Record<string, any>[] = []
  if (length === 1) {
    const cube = appendOperation(null, 'call', 'cube', 'geometry')
    const occurrenceId = referenceSemanticOccurrenceIdV1(
      null,
      null,
      operations[cube].operationId,
      [],
    )
    occurrences.push({
      id: 0, occurrenceId, operation: cube, parent: null, staticParent: null,
      dynamicSlots: [], node: 0, outputOrdinal: 0,
      sceneEntityId: referenceSemanticSceneEntityIdV1(occurrenceId, 0),
    })
  } else {
    const moduleGroup = referenceSemanticAmbiguityGroupIdV1([], 'module', 'chain')
    const definition = appendOperation(
      null, 'module', 'chain', 'module', 0, 'same-name-positional', moduleGroup,
    )
    const definitionBody = appendOperation(definition, 'body', '$body', 'control')
    const blockSize = 200
    const transforms: number[] = []
    const cubes: number[] = []
    let staticParent = definitionBody
    for (let depth = 0; depth < blockSize; depth++) {
      const transform = appendOperation(staticParent, 'call', 'translate', 'transform')
      const cube = appendOperation(transform, 'call', 'cube', 'geometry')
      transforms.push(transform)
      cubes.push(cube)
      staticParent = transform
    }
    const recursiveCall = appendOperation(staticParent, 'call', 'chain', 'module')
    const recursiveBody = appendOperation(recursiveCall, 'body', '$body', 'control')
    appendOperation(recursiveBody, 'control', '$expansion', 'control')
    const initialCall = appendOperation(
      null, 'call', 'chain', 'module', 1, 'same-name-positional', moduleGroup,
    )
    const initialBody = appendOperation(initialCall, 'body', '$body', 'control')
    appendOperation(initialBody, 'control', '$expansion', 'control')

    const appendOccurrence = (
      operation: number,
      parent: number | null,
      occurrenceStaticParent: number | null,
      node: number | null = null,
    ): number => {
      const id = occurrences.length
      const occurrenceId = referenceSemanticOccurrenceIdV1(
        parent === null ? null : occurrences[parent].occurrenceId,
        occurrenceStaticParent === null ? null : occurrences[occurrenceStaticParent].occurrenceId,
        operations[operation].operationId,
        [],
      )
      occurrences.push({
        id,
        occurrenceId,
        operation,
        parent,
        staticParent: occurrenceStaticParent,
        dynamicSlots: [],
        node,
        outputOrdinal: node === null ? null : 0,
        sceneEntityId: node === null ? null : referenceSemanticSceneEntityIdV1(occurrenceId, 0),
      })
      return id
    }

    const initialCallOccurrence = appendOccurrence(initialCall, null, null)
    const initialBodyOccurrence = appendOccurrence(
      initialBody,
      initialCallOccurrence,
      initialCallOccurrence,
    )
    const definitionOccurrence = appendOccurrence(definition, initialBodyOccurrence, null)
    let currentParent = appendOccurrence(
      definitionBody,
      definitionOccurrence,
      definitionOccurrence,
    )
    let currentStaticParent = currentParent
    let blockDepth = 0
    for (let layer = 0; layer < length - 1; layer++) {
      const transformOccurrence = appendOccurrence(
        transforms[blockDepth],
        currentParent,
        currentStaticParent,
        length - 1 - layer,
      )
      if (layer === length - 2) {
        appendOccurrence(
          cubes[blockDepth],
          transformOccurrence,
          transformOccurrence,
          0,
        )
        break
      }
      blockDepth++
      if (blockDepth < blockSize) {
        currentParent = transformOccurrence
        currentStaticParent = transformOccurrence
        continue
      }
      const callOccurrence = appendOccurrence(
        recursiveCall,
        transformOccurrence,
        transformOccurrence,
      )
      const bodyOccurrence = appendOccurrence(recursiveBody, callOccurrence, callOccurrence)
      const recursiveDefinition = appendOccurrence(definition, bodyOccurrence, null)
      currentParent = appendOccurrence(
        definitionBody,
        recursiveDefinition,
        recursiveDefinition,
      )
      currentStaticParent = currentParent
      blockDepth = 0
    }
  }
  const identityOccurrence = occurrences.length - 1
  const producerOccurrence = length === 1 ? 0 : 4
  const program: ReferenceSemanticProgramV1 = {
    schema: 'semantic-program-envelope',
    schemaVersion: { major: 1, minor: 2 },
    source: referenceSemanticSourceDescriptorV1(source),
    core: {
      schema: 'semantic-program-core',
      schemaVersion: { major: 1, minor: 2 },
      requiredFeatures: ['semantic.execution-v2'],
      identityVersion: 'semantic-program-core-v1',
      language: { contract: 'legacy/current', semanticsRevision: '1.0.0', capabilityGraphVersion: 'semantic-capabilities-v1' },
      units: { length: 'millimeter', angle: 'degree', handedness: 'right', upAxis: 'z', matrixLayout: 'column-major', composition: 'parent-times-local' },
      operations,
      occurrences,
      nodes,
      execution: {
        version: 'semantic-execution-v2',
        evaluationOrder: nodes.map(node => node.id),
        discardedEffects: [],
        terminal: null,
      },
      result: {
        tag: 'single',
        item: {
          node: length - 1,
          producerOccurrence,
          identityOccurrence,
          color: [1, 1, 1, 1],
        },
      },
      declaredCapabilities: [],
      capabilityClosure: [],
      diagnosticTemplates: [],
    },
    provenance: operations.map((operation, index) => ({
      operation: index,
      span: { start: 0, end: source.length },
      label: `${operation.name}()`,
    })),
    tessellationIntents: [],
    diagnostics: [],
  }
  program.core.capabilityClosure = referenceSemanticCapabilityClosureV1(program.core)
  return program
}
