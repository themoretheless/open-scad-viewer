import {
  deriveSemanticAmbiguityGroupId,
  deriveSemanticCapabilityClosure,
  deriveSemanticOccurrenceId,
  deriveSemanticOperationId,
  deriveSemanticSceneEntityId,
  SEMANTIC_PROGRAM_CAPABILITY_GRAPH,
  SEMANTIC_PROGRAM_DIAGNOSTIC_CODES,
  SEMANTIC_PROGRAM_EXECUTION_FEATURE,
  SEMANTIC_PROGRAM_EXECUTION_VERSION,
  SEMANTIC_PROGRAM_IDENTITY,
  SEMANTIC_PROGRAM_LIMITS,
  semanticNodeInputs,
  semanticNodeProducerOperationNames,
  semanticResultItems,
  type SemanticDiagnosticTemplate,
  type SemanticDimension,
  type SemanticDynamicSlot,
  type SemanticIdentityValue,
  type SemanticNode,
  type SemanticOccurrence,
  type SemanticOperationCategory,
  type SemanticProgramCoreV1,
  type SemanticProgramDiagnosticCode,
  type SemanticProgramEnvelopeV1,
  type SemanticProgramV1,
  type SemanticSourceDescriptor,
  type SemanticSourceSpan,
  type SemanticStaticOperation,
  type SemanticStructuralPathSegment,
  type SemanticValueType,
} from '../core/semanticProgram'
import { sha256Hex } from '../core/sha256'
import { parseGeometrySourceRoutingHeader } from '../core/geometryRouting'
import { isSemanticProgramCodecOwnedGraph } from './semanticProgramCodec'

export class SemanticProgramValidationError extends TypeError {
  constructor(
    readonly code: SemanticProgramDiagnosticCode,
    readonly path: string,
    detail: string,
  ) {
    super(`${code} at ${path}: ${detail}`)
    this.name = 'SemanticProgramValidationError'
  }
}

function fail(code: SemanticProgramDiagnosticCode, path: string, detail: string): never {
  throw new SemanticProgramValidationError(code, path, detail)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && !Array.isArray(value) && typeof value === 'object'
}

function exactKeys(value: unknown, expected: readonly string[], path: string): asserts value is Record<string, unknown> {
  if (!isRecord(value)) fail('E_SEMANTIC_SCHEMA', path, 'expected an object')
  const actual = Object.keys(value).sort()
  const wanted = [...expected].sort()
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail('E_SEMANTIC_FIELD', path, 'unknown or missing fields')
  }
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

function stringValue(value: unknown, path: string, max = 512): string {
  if (typeof value !== 'string' || !wellFormedUnicode(value)) {
    fail('E_SEMANTIC_FIELD', path, 'expected a well-formed Unicode string')
  }
  if (value.length > max) fail('E_SEMANTIC_LIMIT', path, `string exceeds ${max} UTF-16 code units`)
  return value
}

function finiteNumber(value: unknown, path: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) fail('E_SEMANTIC_NUMBER', path, 'expected a finite binary64 number')
  if (Object.is(value, -0)) fail('E_SEMANTIC_NUMBER', path, '-0 is not canonical; use +0')
  return value
}

function integer(value: unknown, path: string, minimum = 0, maximum = Number.MAX_SAFE_INTEGER): number {
  const parsed = finiteNumber(value, path)
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    fail('E_SEMANTIC_NUMBER', path, `expected an integer in [${minimum}, ${maximum}]`)
  }
  return parsed
}

function positive(value: unknown, path: string, allowZero = false): number {
  const parsed = finiteNumber(value, path)
  if (allowZero ? parsed < 0 : parsed <= 0) fail('E_SEMANTIC_NUMBER', path, allowZero ? 'expected a non-negative number' : 'expected a positive number')
  return parsed
}

function booleanValue(value: unknown, path: string): boolean {
  if (typeof value !== 'boolean') fail('E_SEMANTIC_FIELD', path, 'expected a boolean')
  return value
}

function arrayValue(value: unknown, path: string, maximum: number): unknown[] {
  if (!Array.isArray(value)) fail('E_SEMANTIC_SCHEMA', path, 'expected an array')
  if (value.length > maximum) fail('E_SEMANTIC_LIMIT', path, `array exceeds ${maximum} items`)
  return value
}

function tupleNumbers(value: unknown, length: number, path: string): number[] {
  const items = arrayValue(value, path, length)
  if (items.length !== length) fail('E_SEMANTIC_FIELD', path, `expected exactly ${length} numbers`)
  return items.map((item, index) => finiteNumber(item, `${path}[${index}]`))
}

function nullablePositive(value: unknown, path: string): number | null {
  return value === null ? null : positive(value, path)
}

function assertSchemaVersion(value: unknown, path: string): void {
  exactKeys(value, ['major', 'minor'], path)
  if (value.major === 1 && (value.minor === 0 || value.minor === 1)) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', path, 'semantic programs before 1.2 require exact-source re-lowering')
  }
  if (value.major !== 1 || value.minor !== 2) fail('E_SEMANTIC_VERSION', path, 'only schema version 1.2 is supported')
}

function utf8Compare(left: string, right: string): number {
  const leftBytes = new TextEncoder().encode(left)
  const rightBytes = new TextEncoder().encode(right)
  const shared = Math.min(leftBytes.length, rightBytes.length)
  for (let index = 0; index < shared; index++) {
    if (leftBytes[index] !== rightBytes[index]) return leftBytes[index] - rightBytes[index]
  }
  return leftBytes.length - rightBytes.length
}

function validateSortedStrings(value: unknown, path: string, maximum: number, idPattern: RegExp): readonly string[] {
  const items = arrayValue(value, path, maximum).map((item, index) => {
    const text = stringValue(item, `${path}[${index}]`, 128)
    if (!idPattern.test(text)) fail('E_SEMANTIC_FIELD', `${path}[${index}]`, 'invalid identifier')
    return text
  })
  const sorted = [...new Set(items)].sort(utf8Compare)
  if (sorted.length !== items.length || sorted.some((item, index) => item !== items[index])) {
    fail('E_SEMANTIC_ORDER', path, 'values must be unique and sorted by raw UTF-8 bytes')
  }
  return items
}

function validateIdentityValue(value: unknown, path: string, depth = 0): asserts value is SemanticIdentityValue {
  if (depth > SEMANTIC_PROGRAM_LIMITS.identityValueDepth) fail('E_SEMANTIC_LIMIT', path, 'identity value nesting limit exceeded')
  if (!isRecord(value) || typeof value.tag !== 'string') fail('E_SEMANTIC_SCHEMA', path, 'expected a typed identity value')
  switch (value.tag) {
    case 'undefined':
    case 'null':
      exactKeys(value, ['tag'], path)
      return
    case 'boolean':
      exactKeys(value, ['tag', 'value'], path)
      booleanValue(value.value, `${path}.value`)
      return
    case 'number':
      exactKeys(value, ['tag', 'value'], path)
      finiteNumber(value.value, `${path}.value`)
      return
    case 'string':
      exactKeys(value, ['tag', 'value'], path)
      stringValue(value.value, `${path}.value`, 4_096)
      return
    case 'vector':
      exactKeys(value, ['tag', 'items'], path)
      arrayValue(value.items, `${path}.items`, 100_000)
        .forEach((item, index) => validateIdentityValue(item, `${path}.items[${index}]`, depth + 1))
      return
    default:
      fail('E_SEMANTIC_FIELD', `${path}.tag`, 'unknown identity value tag')
  }
}

function validateStructuralPath(value: unknown, path: string): readonly SemanticStructuralPathSegment[] {
  const segments = arrayValue(value, path, 512)
  if (segments.length === 0) fail('E_SEMANTIC_IDENTITY', path, 'structural path cannot be empty')
  segments.forEach((segment, index) => {
    const segmentPath = `${path}[${index}]`
    exactKeys(segment, ['kind', 'name', 'ordinal'], segmentPath)
    if (!['call', 'module', 'control', 'branch', 'body'].includes(segment.kind as string)) fail('E_SEMANTIC_FIELD', `${segmentPath}.kind`, 'unknown structural path segment kind')
    stringValue(segment.name, `${segmentPath}.name`, 256)
    integer(segment.ordinal, `${segmentPath}.ordinal`, 0, 1_000_000)
  })
  return segments as unknown as readonly SemanticStructuralPathSegment[]
}

function samePath(left: readonly SemanticStructuralPathSegment[], right: readonly SemanticStructuralPathSegment[]): boolean {
  return left.length === right.length && left.every((segment, index) => (
    segment.kind === right[index].kind
    && segment.name === right[index].name
    && segment.ordinal === right[index].ordinal
  ))
}

function structuralPathKey(path: readonly SemanticStructuralPathSegment[]): string {
  return path.map(segment => `${segment.kind.length}:${segment.kind}${segment.name.length}:${segment.name}${segment.ordinal}`).join('|')
}

function validateOperations(value: unknown): readonly SemanticStaticOperation[] {
  const operations = arrayValue(value, '$.core.operations', SEMANTIC_PROGRAM_LIMITS.operations)
  const ambiguityCounts = new Map<string, number>()
  const siblingCounts = new Map<string, number>()
  const siblingOrdinals = new Map<string, number[]>()
  const operationIds = new Set<string>()
  const structuralPaths = new Set<string>()
  const preorderStack: number[] = []
  const closedOperations = new Set<number>()
  operations.forEach((operation, index) => {
    const path = `$.core.operations[${index}]`
    exactKeys(operation, [
      'id', 'operationId', 'parent', 'childOrdinal', 'name', 'category',
      'structuralPath', 'identityEvidence', 'ambiguityGroup',
    ], path)
    if (operation.id !== index) fail('E_SEMANTIC_ORDER', `${path}.id`, 'operation IDs must equal array positions')
    const parent = operation.parent === null ? null : integer(operation.parent, `${path}.parent`, 0, index - 1)
    if (index === 0 && parent !== null) fail('E_SEMANTIC_REFERENCE', `${path}.parent`, 'first operation cannot have a parent')
    while (preorderStack.length > 0 && preorderStack[preorderStack.length - 1] !== parent) {
      closedOperations.add(preorderStack.pop()!)
    }
    if (parent !== null && (closedOperations.has(parent) || preorderStack[preorderStack.length - 1] !== parent)) {
      fail('E_SEMANTIC_ORDER', `${path}.parent`, 'operations must be in deterministic parent-before-child preorder')
    }
    const childOrdinal = integer(operation.childOrdinal, `${path}.childOrdinal`, 0, 1_000_000)
    const name = stringValue(operation.name, `${path}.name`, 256)
    if (!['geometry', 'transform', 'boolean', 'control', 'module', 'assertion', 'presentation'].includes(operation.category as string)) {
      fail('E_SEMANTIC_FIELD', `${path}.category`, 'unknown operation category')
    }
    const category = operation.category as SemanticOperationCategory
    const structuralPath = validateStructuralPath(operation.structuralPath, `${path}.structuralPath`)
    const last = structuralPath[structuralPath.length - 1]
    if (last.name !== name || last.ordinal !== childOrdinal) fail('E_SEMANTIC_IDENTITY', path, 'name and childOrdinal must match the final structural path segment')
    if (parent === null) {
      if (structuralPath.length !== 1) fail('E_SEMANTIC_IDENTITY', `${path}.structuralPath`, 'root operation path must have one segment')
    } else {
      const parentPath = (operations[parent] as SemanticStaticOperation).structuralPath
      if (structuralPath.length !== parentPath.length + 1 || !samePath(structuralPath.slice(0, -1), parentPath)) {
        fail('E_SEMANTIC_IDENTITY', `${path}.structuralPath`, 'child path must extend its parent path exactly once')
      }
    }
    const expectedOperationId = deriveSemanticOperationId(structuralPath)
    if (operation.operationId !== expectedOperationId) fail('E_SEMANTIC_IDENTITY', `${path}.operationId`, 'operation ID does not match structural path')
    const pathKey = structuralPathKey(structuralPath)
    if (structuralPaths.has(pathKey) || operationIds.has(expectedOperationId)) {
      fail('E_SEMANTIC_IDENTITY', `${path}.operationId`, 'static operation paths and operation IDs must be unique')
    }
    structuralPaths.add(pathKey)
    operationIds.add(expectedOperationId)
    const siblingKey = `${String(parent)}\0${category}\0${name}`
    siblingCounts.set(siblingKey, (siblingCounts.get(siblingKey) ?? 0) + 1)
    const ordinals = siblingOrdinals.get(siblingKey) ?? []
    ordinals.push(childOrdinal)
    siblingOrdinals.set(siblingKey, ordinals)
    if (operation.identityEvidence === 'structural-unique') {
      if (operation.ambiguityGroup !== null) fail('E_SEMANTIC_IDENTITY', `${path}.ambiguityGroup`, 'structural-unique operation cannot claim an ambiguity group')
    } else if (operation.identityEvidence === 'same-name-positional') {
      const parentPath = parent === null ? [] : (operations[parent] as SemanticStaticOperation).structuralPath
      const expectedGroup = deriveSemanticAmbiguityGroupId(parentPath, category, name)
      if (operation.ambiguityGroup !== expectedGroup) fail('E_SEMANTIC_IDENTITY', `${path}.ambiguityGroup`, 'ambiguity group does not match sibling identity')
      ambiguityCounts.set(expectedGroup, (ambiguityCounts.get(expectedGroup) ?? 0) + 1)
    } else {
      fail('E_SEMANTIC_FIELD', `${path}.identityEvidence`, 'unknown identity evidence')
    }
    preorderStack.push(index)
  })
  operations.forEach((operation, index) => {
    const siblingKey = `${String((operation as Record<string, unknown>).parent)}\0${String((operation as Record<string, unknown>).category)}\0${String((operation as Record<string, unknown>).name)}`
    const count = siblingCounts.get(siblingKey) ?? 0
    if ((count > 1) !== ((operation as Record<string, unknown>).identityEvidence === 'same-name-positional')) {
      fail('E_SEMANTIC_IDENTITY', `$.core.operations[${index}].identityEvidence`, 'same-name siblings require explicit positional ambiguity evidence')
    }
  })
  for (const [group, count] of ambiguityCounts) {
    if (count < 2) fail('E_SEMANTIC_IDENTITY', '$.core.operations', `ambiguity group ${group} has fewer than two members`)
  }
  for (const [group, ordinals] of siblingOrdinals) {
    if (ordinals.some((ordinal, index) => ordinal !== index)) {
      fail('E_SEMANTIC_ORDER', '$.core.operations', `sibling ordinals for ${group} must be unique and dense from zero`)
    }
  }
  return operations as unknown as readonly SemanticStaticOperation[]
}

function validateValueType(value: unknown, path: string): SemanticValueType {
  exactKeys(value, ['geometryKind', 'space', 'representation', 'evidence'], path)
  if (!['curve', 'wire', 'region', 'sheet', 'solid', 'solid-set'].includes(value.geometryKind as string)) fail('E_SEMANTIC_TYPE', `${path}.geometryKind`, 'unknown geometry kind')
  if (value.space !== 'd2' && value.space !== 'd3') fail('E_SEMANTIC_TYPE', `${path}.space`, 'unknown geometry space')
  const geometryKind = value.geometryKind as SemanticValueType['geometryKind']
  if ((geometryKind === 'region' && value.space !== 'd2')
    || ((geometryKind === 'sheet' || geometryKind === 'solid' || geometryKind === 'solid-set') && value.space !== 'd3')) {
    fail('E_SEMANTIC_TYPE', path, 'geometry kind is incompatible with its ambient space')
  }
  if (!['analytic-brep', 'rational-brep', 'certified-approx-brep', 'mesh'].includes(value.representation as string)) fail('E_SEMANTIC_TYPE', `${path}.representation`, 'unknown representation')
  if (!isRecord(value.evidence) || typeof value.evidence.tag !== 'string') fail('E_SEMANTIC_SCHEMA', `${path}.evidence`, 'expected evidence discriminant')
  if (value.evidence.tag === 'representation-preserving') {
    exactKeys(value.evidence, ['tag'], `${path}.evidence`)
    if (value.representation === 'certified-approx-brep') fail('E_SEMANTIC_TYPE', path, 'certified approximation representation requires certified evidence')
  } else if (value.evidence.tag === 'certified-approximation') {
    exactKeys(value.evidence, ['tag', 'certificateProfile', 'certificatePolicyHash'], `${path}.evidence`)
    const profile = stringValue(value.evidence.certificateProfile, `${path}.evidence.certificateProfile`, 96)
    if (!/^[A-Za-z0-9][A-Za-z0-9._:-]{0,95}$/.test(profile)) fail('E_SEMANTIC_FIELD', `${path}.evidence.certificateProfile`, 'invalid certificate profile')
    if (typeof value.evidence.certificatePolicyHash !== 'string' || !/^[a-f0-9]{64}$/.test(value.evidence.certificatePolicyHash)) fail('E_SEMANTIC_FIELD', `${path}.evidence.certificatePolicyHash`, 'expected lowercase SHA-256')
    if (value.representation !== 'certified-approx-brep') fail('E_SEMANTIC_TYPE', path, 'certified evidence requires certified approximation representation')
  } else {
    fail('E_SEMANTIC_FIELD', `${path}.evidence.tag`, 'unknown representation evidence')
  }
  return value as unknown as SemanticValueType
}

function representationEvidenceEqual(left: SemanticValueType, right: SemanticValueType): boolean {
  if (left.representation !== right.representation || left.evidence.tag !== right.evidence.tag) return false
  return left.evidence.tag === 'representation-preserving'
    || (right.evidence.tag === 'certified-approximation'
      && left.evidence.certificateProfile === right.evidence.certificateProfile
      && left.evidence.certificatePolicyHash === right.evidence.certificatePolicyHash)
}

function sameValueType(left: SemanticValueType, right: SemanticValueType): boolean {
  return left.geometryKind === right.geometryKind && left.space === right.space
    && representationEvidenceEqual(left, right)
}

function expectedValueType(
  contract: string,
  geometryKind: SemanticValueType['geometryKind'],
  space: SemanticValueType['space'],
): SemanticValueType {
  return {
    geometryKind,
    space,
    representation: contract === 'legacy/current' ? 'mesh' : 'analytic-brep',
    evidence: { tag: 'representation-preserving' },
  }
}

function assertValueType(actual: SemanticValueType, expected: SemanticValueType, path: string): void {
  if (!sameValueType(actual, expected)) fail('E_SEMANTIC_TYPE', path, 'node valueType does not match its semantic transition')
}

function nodeReference(value: unknown, path: string, before: number): number {
  const reference = integer(value, path, 0, SEMANTIC_PROGRAM_LIMITS.nodes - 1)
  if (reference >= before) fail('E_SEMANTIC_REFERENCE', path, 'DAG references must point to an earlier node')
  return reference
}

function validateAnalyticLanguage(kind: string, contract: string, path: string): void {
  const analytic = kind.endsWith('-analytic')
  const polygonal = kind.endsWith('-polygonal')
  if (contract === 'legacy/current' && analytic) fail('E_SEMANTIC_TYPE', path, 'legacy/current requires materialized polygonal geometry')
  if (contract === 'openscad-viewer/brep-1' && polygonal) fail('E_SEMANTIC_TYPE', path, 'brep-1 topology requires analytic geometry')
}

function validateNodes(value: unknown, contract: string): readonly SemanticNode[] {
  const nodes = arrayValue(value, '$.core.nodes', SEMANTIC_PROGRAM_LIMITS.nodes)
  nodes.forEach((node, index) => {
    const path = `$.core.nodes[${index}]`
    if (!isRecord(node)) fail('E_SEMANTIC_SCHEMA', path, 'expected a node object')
    if (node.id !== index) fail('E_SEMANTIC_ORDER', `${path}.id`, 'node IDs must equal canonical array positions')
    const kind = stringValue(node.kind, `${path}.kind`, 64)
    validateAnalyticLanguage(kind, contract, `${path}.kind`)
    const valueType = validateValueType(node.valueType, `${path}.valueType`)
    switch (kind) {
      case 'box':
        exactKeys(node, ['id', 'kind', 'valueType', 'size', 'center'], path)
        assertValueType(valueType, expectedValueType(contract, contract === 'legacy/current' ? 'solid-set' : 'solid', 'd3'), `${path}.valueType`)
        if (tupleNumbers(node.size, 3, `${path}.size`).some(item => item <= 0)) fail('E_SEMANTIC_NUMBER', `${path}.size`, 'box sizes must be positive')
        booleanValue(node.center, `${path}.center`)
        break
      case 'sphere-analytic':
      case 'sphere-polygonal':
        exactKeys(node, kind === 'sphere-analytic'
          ? ['id', 'kind', 'valueType', 'radius']
          : ['id', 'kind', 'valueType', 'radius', 'radialSegments'], path)
        assertValueType(valueType, expectedValueType(contract, contract === 'legacy/current' ? 'solid-set' : 'solid', 'd3'), `${path}.valueType`)
        positive(node.radius, `${path}.radius`)
        if (kind === 'sphere-polygonal') integer(node.radialSegments, `${path}.radialSegments`, 4, 1_000_000)
        break
      case 'cylinder-analytic':
      case 'cylinder-polygonal': {
        exactKeys(node, kind === 'cylinder-analytic'
          ? ['id', 'kind', 'valueType', 'height', 'radiusBottom', 'radiusTop', 'center']
          : ['id', 'kind', 'valueType', 'height', 'radiusBottom', 'radiusTop', 'center', 'radialSegments'], path)
        assertValueType(valueType, expectedValueType(contract, contract === 'legacy/current' ? 'solid-set' : 'solid', 'd3'), `${path}.valueType`)
        positive(node.height, `${path}.height`)
        const bottom = positive(node.radiusBottom, `${path}.radiusBottom`, true)
        const top = positive(node.radiusTop, `${path}.radiusTop`, true)
        if (bottom === 0 && top === 0) fail('E_SEMANTIC_NUMBER', path, 'cylinder radii cannot both be zero')
        booleanValue(node.center, `${path}.center`)
        if (kind === 'cylinder-polygonal') integer(node.radialSegments, `${path}.radialSegments`, 3, 1_000_000)
        break
      }
      case 'polyhedron': {
        exactKeys(node, ['id', 'kind', 'valueType', 'vertices', 'triangles'], path)
        assertValueType(valueType, expectedValueType(contract, 'solid-set', 'd3'), `${path}.valueType`)
        const vertices = arrayValue(node.vertices, `${path}.vertices`, 750_000)
        const triangles = arrayValue(node.triangles, `${path}.triangles`, 750_000)
        const legacyMaterializedEmpty = contract === 'legacy/current'
          && vertices.length === 0
          && triangles.length === 0
        if (vertices.length < 4 && !legacyMaterializedEmpty) {
          fail('E_SEMANTIC_TYPE', `${path}.vertices`, 'solid polyhedron requires at least four vertices or the exact legacy empty form')
        }
        vertices.forEach((vertex, vertexIndex) => tupleNumbers(vertex, 3, `${path}.vertices[${vertexIndex}]`))
        triangles.forEach((triangle, triangleIndex) => {
          const values = arrayValue(triangle, `${path}.triangles[${triangleIndex}]`, 3)
          if (values.length !== 3) fail('E_SEMANTIC_FIELD', `${path}.triangles[${triangleIndex}]`, 'triangle requires three indices')
          const indices = values.map((item, component) => integer(item, `${path}.triangles[${triangleIndex}][${component}]`, 0, vertices.length - 1))
          if (contract !== 'legacy/current' && new Set(indices).size !== 3) {
            fail('E_SEMANTIC_TYPE', `${path}.triangles[${triangleIndex}]`, 'triangle indices must be distinct')
          }
        })
        break
      }
      case 'rectangle':
        exactKeys(node, ['id', 'kind', 'valueType', 'size', 'center'], path)
        assertValueType(valueType, expectedValueType(contract, 'region', 'd2'), `${path}.valueType`)
        if (tupleNumbers(node.size, 2, `${path}.size`).some(item => item <= 0)) fail('E_SEMANTIC_NUMBER', `${path}.size`, 'rectangle sizes must be positive')
        booleanValue(node.center, `${path}.center`)
        break
      case 'circle-analytic':
      case 'circle-polygonal':
        exactKeys(node, kind === 'circle-analytic'
          ? ['id', 'kind', 'valueType', 'radius']
          : ['id', 'kind', 'valueType', 'radius', 'radialSegments'], path)
        assertValueType(valueType, expectedValueType(contract, 'region', 'd2'), `${path}.valueType`)
        positive(node.radius, `${path}.radius`)
        if (kind === 'circle-polygonal') integer(node.radialSegments, `${path}.radialSegments`, 3, 1_000_000)
        break
      case 'polygon': {
        exactKeys(node, ['id', 'kind', 'valueType', 'rings', 'fillRule'], path)
        assertValueType(valueType, expectedValueType(contract, 'region', 'd2'), `${path}.valueType`)
        if (node.fillRule !== 'even-odd') fail('E_SEMANTIC_FIELD', `${path}.fillRule`, 'v1 defines even-odd only')
        const rings = arrayValue(node.rings, `${path}.rings`, 100_000)
        if (rings.length === 0) fail('E_SEMANTIC_FIELD', `${path}.rings`, 'polygon needs at least one ring')
        rings.forEach((ring, ringIndex) => {
          const points = arrayValue(ring, `${path}.rings[${ringIndex}]`, 1_000_000)
          if (contract !== 'legacy/current' && points.length < 3) {
            fail('E_SEMANTIC_FIELD', `${path}.rings[${ringIndex}]`, 'ring needs at least three points')
          }
          points.forEach((point, pointIndex) => tupleNumbers(point, 2, `${path}.rings[${ringIndex}][${pointIndex}]`))
        })
        break
      }
      case 'transform': {
        exactKeys(node, ['id', 'kind', 'valueType', 'input', 'matrix'], path)
        const input = nodeReference(node.input, `${path}.input`, index)
        assertValueType(valueType, (nodes[input] as SemanticNode).valueType, `${path}.valueType`)
        const matrix = tupleNumbers(node.matrix, 16, `${path}.matrix`)
        if (matrix[3] !== 0 || matrix[7] !== 0 || matrix[11] !== 0 || matrix[15] !== 1) fail('E_SEMANTIC_TYPE', `${path}.matrix`, 'matrix must be affine column-major')
        const determinant = matrix[0] * (matrix[5] * matrix[10] - matrix[9] * matrix[6])
          - matrix[4] * (matrix[1] * matrix[10] - matrix[9] * matrix[2])
          + matrix[8] * (matrix[1] * matrix[6] - matrix[5] * matrix[2])
        if (contract === 'openscad-viewer/brep-1' && (!Number.isFinite(determinant) || determinant === 0)) {
          fail('E_SEMANTIC_TYPE', `${path}.matrix`, 'brep-1 transform linear part must be nonsingular; reflection is allowed')
        }
        break
      }
      case 'boolean':
      case 'hull': {
        exactKeys(node, kind === 'boolean' ? ['id', 'kind', 'valueType', 'operation', 'inputs'] : ['id', 'kind', 'valueType', 'inputs'], path)
        if (kind === 'boolean' && !['union', 'intersection', 'difference'].includes(node.operation as string)) fail('E_SEMANTIC_FIELD', `${path}.operation`, 'unknown Boolean operation')
        const inputs = arrayValue(node.inputs, `${path}.inputs`, SEMANTIC_PROGRAM_LIMITS.nodes)
        if (inputs.length < 2) fail('E_SEMANTIC_TYPE', `${path}.inputs`, 'zero operands lower to empty and one operand lowers to an alias')
        const references = inputs.map((input, inputIndex) => nodeReference(input, `${path}.inputs[${inputIndex}]`, index))
        const firstType = (nodes[references[0]] as SemanticNode).valueType
        references.forEach((reference, inputIndex) => {
          const candidate = (nodes[reference] as SemanticNode).valueType
          if (candidate.space !== firstType.space || !representationEvidenceEqual(candidate, firstType)) fail('E_SEMANTIC_TYPE', `${path}.inputs[${inputIndex}]`, 'operands must share space, representation, and evidence')
          if (firstType.space === 'd2' ? candidate.geometryKind !== 'region' : candidate.geometryKind !== 'solid' && candidate.geometryKind !== 'solid-set') {
            fail('E_SEMANTIC_TYPE', `${path}.inputs[${inputIndex}]`, 'operand geometry kind is not admitted by this operation')
          }
        })
        const expected: SemanticValueType = {
          geometryKind: firstType.space === 'd2' ? 'region' : 'solid-set',
          space: firstType.space,
          representation: firstType.representation,
          evidence: firstType.evidence,
        }
        assertValueType(valueType, expected, `${path}.valueType`)
        break
      }
      case 'linear-extrude': {
        exactKeys(node, ['id', 'kind', 'valueType', 'input', 'height', 'twistDegrees', 'slices', 'scale', 'center'], path)
        const input = nodeReference(node.input, `${path}.input`, index)
        const inputType = (nodes[input] as SemanticNode).valueType
        if (inputType.geometryKind !== 'region' || inputType.space !== 'd2') fail('E_SEMANTIC_TYPE', `${path}.input`, 'linear-extrude consumes Region/d2')
        assertValueType(valueType, { ...inputType, geometryKind: 'solid-set', space: 'd3' }, `${path}.valueType`)
        positive(node.height, `${path}.height`)
        finiteNumber(node.twistDegrees, `${path}.twistDegrees`)
        integer(node.slices, `${path}.slices`, 0, 1_000_000)
        const scale = tupleNumbers(node.scale, 2, `${path}.scale`)
        if (contract === 'openscad-viewer/brep-1' && scale.some(item => item === 0)) {
          fail('E_SEMANTIC_NUMBER', `${path}.scale`, 'brep-1 extrusion scale cannot contain zero')
        }
        booleanValue(node.center, `${path}.center`)
        break
      }
      case 'rotate-extrude-analytic':
      case 'rotate-extrude-polygonal': {
        exactKeys(node, kind === 'rotate-extrude-analytic'
          ? ['id', 'kind', 'valueType', 'input', 'angleDegrees']
          : ['id', 'kind', 'valueType', 'input', 'angleDegrees', 'radialSegments'], path)
        const input = nodeReference(node.input, `${path}.input`, index)
        const inputType = (nodes[input] as SemanticNode).valueType
        if (inputType.geometryKind !== 'region' || inputType.space !== 'd2') fail('E_SEMANTIC_TYPE', `${path}.input`, 'rotate-extrude consumes Region/d2')
        assertValueType(valueType, { ...inputType, geometryKind: 'solid-set', space: 'd3' }, `${path}.valueType`)
        const angle = contract === 'legacy/current'
          ? finiteNumber(node.angleDegrees, `${path}.angleDegrees`)
          : positive(node.angleDegrees, `${path}.angleDegrees`)
        if (contract === 'openscad-viewer/brep-1' && angle > 360) fail('E_SEMANTIC_NUMBER', `${path}.angleDegrees`, 'angle cannot exceed 360 degrees')
        if (kind === 'rotate-extrude-polygonal') integer(node.radialSegments, `${path}.radialSegments`, 3, 1_000_000)
        break
      }
      case 'projection': {
        exactKeys(node, ['id', 'kind', 'valueType', 'input', 'cut'], path)
        const input = nodeReference(node.input, `${path}.input`, index)
        const inputType = (nodes[input] as SemanticNode).valueType
        if (inputType.space !== 'd3' || (inputType.geometryKind !== 'solid' && inputType.geometryKind !== 'solid-set')) fail('E_SEMANTIC_TYPE', `${path}.input`, 'projection consumes Solid|SolidSet/d3')
        assertValueType(valueType, { ...inputType, geometryKind: 'region', space: 'd2' }, `${path}.valueType`)
        booleanValue(node.cut, `${path}.cut`)
        break
      }
      case 'offset': {
        exactKeys(node, ['id', 'kind', 'valueType', 'input', 'distance'], path)
        const input = nodeReference(node.input, `${path}.input`, index)
        const inputType = (nodes[input] as SemanticNode).valueType
        if (inputType.geometryKind !== 'region' || inputType.space !== 'd2') fail('E_SEMANTIC_TYPE', `${path}.input`, 'offset consumes Region/d2')
        assertValueType(valueType, inputType, `${path}.valueType`)
        finiteNumber(node.distance, `${path}.distance`)
        break
      }
      default:
        fail('E_SEMANTIC_FIELD', `${path}.kind`, `unknown node kind ${JSON.stringify(kind)}`)
    }
  })
  return nodes as unknown as readonly SemanticNode[]
}

function identityValueKey(value: SemanticIdentityValue): string {
  switch (value.tag) {
    case 'undefined': return 'u'
    case 'null': return 'n'
    case 'boolean': return value.value ? 'b1' : 'b0'
    case 'number': return `d${String(value.value)}`
    case 'string': return `s${value.value.length}:${value.value}`
    case 'vector': return `v${value.items.length}[${value.items.map(identityValueKey).join(',')}]`
  }
}

function sameDynamicSlots(left: readonly SemanticDynamicSlot[], right: readonly SemanticDynamicSlot[]): boolean {
  return left.length === right.length && left.every((slot, index) => (
    slot.name === right[index].name
    && slot.duplicateOrdinal === right[index].duplicateOrdinal
    && identityValueKey(slot.value) === identityValueKey(right[index].value)
  ))
}

function sameDynamicBindings(left: readonly SemanticDynamicSlot[], right: readonly SemanticDynamicSlot[]): boolean {
  return left.length === right.length && left.every((slot, index) => (
    slot.name === right[index].name
    && identityValueKey(slot.value) === identityValueKey(right[index].value)
  ))
}

interface SemanticModuleActivationAnchor {
  readonly callOccurrence: number
  readonly bodyOccurrence: number
  readonly definitionOccurrence: number
  readonly expansionOperation: number
}

interface SemanticOccurrenceValidationIndex {
  readonly operationChildren: readonly (readonly number[])[]
  readonly runtimeChildren: readonly (readonly number[])[]
  readonly activationByDefinition: Map<number, SemanticModuleActivationAnchor | null>
  readonly proofBudget: { steps: number }
}

function consumeOccurrenceProofBudget(
  validationIndex: Pick<SemanticOccurrenceValidationIndex, 'proofBudget'>,
  steps = 1,
): void {
  validationIndex.proofBudget.steps += steps
  if (validationIndex.proofBudget.steps > SEMANTIC_PROGRAM_LIMITS.snapshotValues) {
    fail('E_SEMANTIC_LIMIT', '$.core.occurrences', 'occurrence ancestry/continuation proof exceeds its bounded work budget')
  }
}

function moduleActivationAnchor(
  definitionOccurrenceIndex: number,
  occurrences: readonly unknown[],
  operations: readonly SemanticStaticOperation[],
  validationIndex: SemanticOccurrenceValidationIndex,
): SemanticModuleActivationAnchor | null {
  const cached = validationIndex.activationByDefinition.get(definitionOccurrenceIndex)
  if (cached !== undefined) return cached
  const reject = (): null => {
    validationIndex.activationByDefinition.set(definitionOccurrenceIndex, null)
    return null
  }
  const definitionOccurrence = occurrences[definitionOccurrenceIndex] as SemanticOccurrence
  const definitionOperation = operations[definitionOccurrence.operation]
  if (definitionOperation.parent !== null
    || definitionOperation.category !== 'module'
    || definitionOperation.name.startsWith('$')
    || definitionOperation.structuralPath.at(-1)?.kind !== 'module') return reject()
  const bodyOccurrenceIndex = definitionOccurrence.parent
  if (bodyOccurrenceIndex === null) return reject()
  const bodyOccurrence = occurrences[bodyOccurrenceIndex] as SemanticOccurrence
  const bodyOperation = operations[bodyOccurrence.operation]
  const bodySegment = bodyOperation.structuralPath.at(-1)
  if (bodyOperation.category !== 'control'
    || bodyOperation.name !== '$body'
    || bodySegment?.kind !== 'body'
    || bodySegment.name !== '$body'
    || bodySegment.ordinal !== 0
    || bodyOperation.parent === null) return reject()
  const callOccurrenceIndex = bodyOccurrence.parent
  if (callOccurrenceIndex === null) return reject()
  const callOccurrence = occurrences[callOccurrenceIndex] as SemanticOccurrence
  const callOperation = operations[callOccurrence.operation]
  if (bodyOperation.parent !== callOccurrence.operation
    || callOperation.category !== 'module'
    || callOperation.name.startsWith('$')
    || callOperation.structuralPath.at(-1)?.kind !== 'call'
    || callOperation.name !== definitionOperation.name
    || !sameDynamicBindings(callOccurrence.dynamicSlots, definitionOccurrence.dynamicSlots)) return reject()

  const staticBodies = validationIndex.operationChildren[callOccurrence.operation] ?? []
  const staticExpansions = validationIndex.operationChildren[bodyOccurrence.operation] ?? []
  consumeOccurrenceProofBudget(validationIndex, staticBodies.length + staticExpansions.length)
  if (staticBodies.length !== 1 || staticBodies[0] !== bodyOccurrence.operation
    || staticExpansions.length !== 1) return reject()
  const soleStaticExpansion = staticExpansions[0]
  const expansionOperation = operations[soleStaticExpansion]
  const expansionSegment = expansionOperation.structuralPath.at(-1)
  if (expansionOperation.category !== 'control'
    || expansionOperation.name !== '$expansion'
    || expansionSegment?.kind !== 'control'
    || expansionSegment.name !== '$expansion'
    || expansionSegment.ordinal !== 0) return reject()

  const callRuntimeChildren = validationIndex.runtimeChildren[callOccurrenceIndex] ?? []
  const bodyRuntimeChildren = validationIndex.runtimeChildren[bodyOccurrenceIndex] ?? []
  consumeOccurrenceProofBudget(validationIndex, callRuntimeChildren.length + bodyRuntimeChildren.length)
  if (callRuntimeChildren.length !== 1 || callRuntimeChildren[0] !== bodyOccurrenceIndex
    || bodyRuntimeChildren.length !== 1 || bodyRuntimeChildren[0] !== definitionOccurrenceIndex) return reject()
  const anchor = {
    callOccurrence: callOccurrenceIndex,
    bodyOccurrence: bodyOccurrenceIndex,
    definitionOccurrence: definitionOccurrenceIndex,
    expansionOperation: soleStaticExpansion,
  }
  validationIndex.activationByDefinition.set(definitionOccurrenceIndex, anchor)
  return anchor
}

function isChildrenExpansionContinuation(
  occurrenceIndex: number,
  operation: number,
  parent: number | null,
  staticParent: number,
  slots: readonly SemanticDynamicSlot[],
  occurrences: readonly unknown[],
  operations: readonly SemanticStaticOperation[],
  validationIndex: SemanticOccurrenceValidationIndex,
): boolean {
  if (parent === null) return false
  const expansionOperation = operations[operation]
  const childrenOccurrence = occurrences[parent] as SemanticOccurrence
  const childrenOperation = operations[childrenOccurrence.operation]
  const callerBodyOccurrence = occurrences[staticParent] as SemanticOccurrence
  const expansionSegment = expansionOperation.structuralPath.at(-1)
  const childrenSegment = childrenOperation.structuralPath.at(-1)
  if (expansionOperation.name !== '$expansion'
    || expansionOperation.category !== 'control'
    || expansionSegment?.kind !== 'control'
    || expansionSegment.name !== '$expansion'
    || expansionSegment.ordinal !== 0
    || expansionOperation.parent !== callerBodyOccurrence.operation
    || childrenOperation.name !== 'children'
    || childrenOperation.category !== 'control'
    || childrenSegment?.kind !== 'control'
    || childrenSegment.name !== 'children'
  ) return false

  const childrenSlots = childrenOccurrence.dynamicSlots
  if (childrenSlots.length !== 1
    || slots.length !== 1
    || childrenSlots[0].name !== '$index'
    || slots[0].name !== '$index'
    || identityValueKey(childrenSlots[0].value) !== identityValueKey(slots[0].value)) return false

  // A selected `!children()` is detached from the module invocation that
  // supplied its dynamic continuation. The lowerer represents each bounded
  // continuation as the sole static/runtime $expansion directly beneath its
  // consuming children() call; all expanded statements must stay in that
  // exact static subtree.
  if (staticParent === parent
    && callerBodyOccurrence.operation === childrenOccurrence.operation) {
    const staticChildren = validationIndex.operationChildren[childrenOccurrence.operation] ?? []
    const runtimeChildren = validationIndex.runtimeChildren[parent] ?? []
    const expandedChildren = validationIndex.runtimeChildren[occurrenceIndex] ?? []
    consumeOccurrenceProofBudget(
      validationIndex,
      staticChildren.length + runtimeChildren.length + expandedChildren.length,
    )
    return staticChildren.length === 1
      && staticChildren[0] === operation
      && runtimeChildren.length === 1
      && runtimeChildren[0] === occurrenceIndex
      && expandedChildren.every(candidate => (
        operations[(occurrences[candidate] as SemanticOccurrence).operation].parent === operation
      ))
  }

  let cursor = childrenOccurrence.parent
  let definitionOccurrence: SemanticOccurrence | null = null
  while (cursor !== null && cursor !== staticParent) {
    const candidate = occurrences[cursor] as SemanticOccurrence
    const candidateOperation = operations[candidate.operation]
    const candidatePath = candidateOperation.structuralPath
    const childrenPath = childrenOperation.structuralPath
    consumeOccurrenceProofBudget(validationIndex, candidatePath.length)
    const lexicallyContainsChildren = candidatePath.length < childrenPath.length
      && candidatePath.every((segment, index) => {
        const childSegment = childrenPath[index]
        return segment.kind === childSegment.kind
          && segment.name === childSegment.name
          && segment.ordinal === childSegment.ordinal
      })
    if (candidateOperation.structuralPath.at(-1)?.kind === 'module'
      && lexicallyContainsChildren) {
      definitionOccurrence = candidate
      break
    }
    consumeOccurrenceProofBudget(validationIndex)
    cursor = candidate.parent
  }
  if (!definitionOccurrence || definitionOccurrence.parent !== staticParent) return false
  const definitionOperation = operations[definitionOccurrence.operation]
  const activation = moduleActivationAnchor(definitionOccurrence.id, occurrences, operations, validationIndex)
  if (!activation
    || activation.bodyOccurrence !== staticParent
    || activation.expansionOperation !== operation) return false
  const definitionPath = definitionOperation.structuralPath
  const childrenPath = childrenOperation.structuralPath
  consumeOccurrenceProofBudget(validationIndex, definitionPath.length)
  const lexicallyContainsChildren = definitionPath.length < childrenPath.length
    && definitionPath.every((segment, index) => {
      const candidate = childrenPath[index]
      return segment.kind === candidate.kind
        && segment.name === candidate.name
        && segment.ordinal === candidate.ordinal
    })
  const expansionRuntimeChildren = validationIndex.runtimeChildren[parent] ?? []
  const expandedChildren = validationIndex.runtimeChildren[occurrenceIndex] ?? []
  consumeOccurrenceProofBudget(validationIndex, expansionRuntimeChildren.length + expandedChildren.length)
  const expandedChildrenStayInStaticSubtree = expandedChildren.every(candidate => (
    operations[(occurrences[candidate] as SemanticOccurrence).operation].parent === operation
  ))
  return lexicallyContainsChildren
    && expansionRuntimeChildren.length === 1
    && expansionRuntimeChildren[0] === occurrenceIndex
    && expandedChildrenStayInStaticSubtree
}

function validateOccurrences(
  value: unknown,
  operations: readonly SemanticStaticOperation[],
  nodes: readonly SemanticNode[],
): readonly SemanticOccurrence[] {
  const occurrences = arrayValue(value, '$.core.occurrences', SEMANTIC_PROGRAM_LIMITS.occurrences)
  const sceneIds = new Set<string>()
  const occurrenceGroups = new Map<string, number[]>()
  const duplicateSlotGroups = new Map<string, Map<number, string>>()
  const reparentedStaticRoots: number[] = []
  const pendingContinuations: Array<Readonly<{
    occurrence: number
    operation: number
    parent: number | null
    staticParent: number
    slots: readonly SemanticDynamicSlot[]
  }>> = []
  const proofBudget = { steps: 0 }
  occurrences.forEach((occurrence, index) => {
    const path = `$.core.occurrences[${index}]`
    exactKeys(occurrence, [
      'id', 'occurrenceId', 'operation', 'parent', 'staticParent', 'dynamicSlots', 'node',
      'outputOrdinal', 'sceneEntityId',
    ], path)
    if (occurrence.id !== index) fail('E_SEMANTIC_ORDER', `${path}.id`, 'occurrence IDs must equal array positions')
    const operation = integer(occurrence.operation, `${path}.operation`, 0, Math.max(0, operations.length - 1))
    if (operations.length === 0) fail('E_SEMANTIC_REFERENCE', `${path}.operation`, 'occurrence requires an operation')
    const parent = occurrence.parent === null ? null : integer(occurrence.parent, `${path}.parent`, 0, index - 1)
    const staticParent = operations[operation].parent
    const staticParentOccurrence = occurrence.staticParent === null
      ? null
      : integer(occurrence.staticParent, `${path}.staticParent`, 0, index - 1)
    const slots = arrayValue(occurrence.dynamicSlots, `${path}.dynamicSlots`, 1_000)
    const slotNames = new Set<string>()
    slots.forEach((slot, slotIndex) => {
      const slotPath = `${path}.dynamicSlots[${slotIndex}]`
      exactKeys(slot, ['name', 'value', 'duplicateOrdinal'], slotPath)
      const name = stringValue(slot.name, `${slotPath}.name`, 128)
      if (slotNames.has(name)) fail('E_SEMANTIC_IDENTITY', `${slotPath}.name`, 'dynamic slot names must be unique in authored order')
      slotNames.add(name)
      validateIdentityValue(slot.value, `${slotPath}.value`)
      integer(slot.duplicateOrdinal, `${slotPath}.duplicateOrdinal`, 0, 1_000_000)
    })
    if (staticParent === null) {
      if (staticParentOccurrence !== null) fail('E_SEMANTIC_IDENTITY', `${path}.staticParent`, 'root static operation requires null staticParent')
      if (parent !== null) {
        const rootOperation = operations[operation]
        if (rootOperation.category !== 'module'
          || rootOperation.structuralPath.at(-1)?.kind !== 'module') {
          fail('E_SEMANTIC_IDENTITY', `${path}.parent`, 'a static-root operation cannot have a runtime parent outside a module-definition activation')
        }
        reparentedStaticRoots.push(index)
      }
    } else {
      if (staticParentOccurrence === null
        || (occurrences[staticParentOccurrence] as SemanticOccurrence).operation !== staticParent) {
        fail('E_SEMANTIC_IDENTITY', `${path}.staticParent`, 'staticParent must instantiate the static parent operation')
      }
      let cursor = parent
      let crossedNonExpansionFrame = false
      while (cursor !== staticParentOccurrence) {
        consumeOccurrenceProofBudget({ proofBudget })
        if (cursor === null) fail('E_SEMANTIC_IDENTITY', `${path}.staticParent`, 'staticParent is not on the runtime ancestor chain')
        const skipped = occurrences[cursor] as SemanticOccurrence
        if (skipped.operation === staticParent) fail('E_SEMANTIC_IDENTITY', `${path}.staticParent`, 'staticParent must be the nearest matching ancestor')
        const category = operations[skipped.operation].category
        if (category !== 'control' && category !== 'module') crossedNonExpansionFrame = true
        cursor = skipped.parent
      }
      const expansion = operations[operation].name === '$expansion'
      if (expansion) {
        pendingContinuations.push(Object.freeze({
          occurrence: index,
          operation,
          parent,
          staticParent: staticParentOccurrence,
          slots: slots as unknown as readonly SemanticDynamicSlot[],
        }))
      } else if (crossedNonExpansionFrame) {
        fail('E_SEMANTIC_IDENTITY', `${path}.parent`, 'only control/module frames may separate parent from staticParent')
      }
    }
    const parentOccurrenceId = parent === null ? null : (occurrences[parent] as SemanticOccurrence).occurrenceId
    const staticParentOccurrenceId = staticParentOccurrence === null
      ? null
      : (occurrences[staticParentOccurrence] as SemanticOccurrence).occurrenceId
    const expectedOccurrenceId = deriveSemanticOccurrenceId(
      parentOccurrenceId,
      staticParentOccurrenceId,
      operations[operation].operationId,
      slots as unknown as readonly SemanticDynamicSlot[],
    )
    if (occurrence.occurrenceId !== expectedOccurrenceId) fail('E_SEMANTIC_IDENTITY', `${path}.occurrenceId`, 'occurrence ID does not match operation, parent, and dynamic slots')
    const group = occurrenceGroups.get(expectedOccurrenceId) ?? []
    if (group.length > 0) {
      const first = occurrences[group[0]] as SemanticOccurrence
      if (first.operation !== operation || first.parent !== parent
        || first.staticParent !== staticParentOccurrence
        || !sameDynamicSlots(first.dynamicSlots, slots as unknown as readonly SemanticDynamicSlot[])) {
        fail('E_SEMANTIC_IDENTITY', path, 'rows sharing an occurrence ID must describe the same logical evaluation')
      }
    }
    group.push(index)
    occurrenceGroups.set(expectedOccurrenceId, group)
    ;(slots as unknown as readonly SemanticDynamicSlot[]).forEach((slot, slotIndex) => {
      const valueKey = identityValueKey(slot.value)
      const duplicateKey = `${String(parent)}|${operation}|${slotIndex}|${slot.name.length}:${slot.name}${valueKey.length}:${valueKey}`
      const ordinals = duplicateSlotGroups.get(duplicateKey) ?? new Map<number, string>()
      const existing = ordinals.get(slot.duplicateOrdinal)
      if (existing !== undefined && existing !== expectedOccurrenceId) {
        fail('E_SEMANTIC_IDENTITY', `${path}.dynamicSlots[${slotIndex}].duplicateOrdinal`, 'duplicate ordinals must distinguish equal dynamic values')
      }
      if (existing === undefined && slot.duplicateOrdinal !== ordinals.size) {
        fail('E_SEMANTIC_ORDER', `${path}.dynamicSlots[${slotIndex}].duplicateOrdinal`, 'duplicate ordinals must be dense in language evaluation order')
      }
      ordinals.set(slot.duplicateOrdinal, expectedOccurrenceId)
      duplicateSlotGroups.set(duplicateKey, ordinals)
    })
    if (occurrence.node === null) {
      if (occurrence.outputOrdinal !== null || occurrence.sceneEntityId !== null) fail('E_SEMANTIC_IDENTITY', path, 'non-producing occurrence cannot own an output identity')
    } else {
      integer(occurrence.node, `${path}.node`, 0, Math.max(0, nodes.length - 1))
      if (nodes.length === 0) fail('E_SEMANTIC_REFERENCE', `${path}.node`, 'occurrence references absent node')
      if (occurrence.outputOrdinal === null) {
        if (occurrence.sceneEntityId !== null) fail('E_SEMANTIC_IDENTITY', `${path}.sceneEntityId`, 'producer-only occurrence cannot claim scene identity')
      } else {
        const outputOrdinal = integer(occurrence.outputOrdinal, `${path}.outputOrdinal`, 0, 1_000_000)
        const expectedSceneId = deriveSemanticSceneEntityId(expectedOccurrenceId, outputOrdinal)
        if (occurrence.sceneEntityId !== expectedSceneId) fail('E_SEMANTIC_IDENTITY', `${path}.sceneEntityId`, 'scene entity ID does not match occurrence and output ordinal')
        if (sceneIds.has(expectedSceneId)) fail('E_SEMANTIC_IDENTITY', `${path}.sceneEntityId`, 'scene entity IDs must be unique')
        sceneIds.add(expectedSceneId)
      }
    }
  })

  const operationChildren: number[][] = Array.from({ length: operations.length }, () => [])
  operations.forEach(operation => {
    if (operation.parent !== null) operationChildren[operation.parent].push(operation.id)
  })
  const runtimeChildren: number[][] = Array.from({ length: occurrences.length }, () => [])
  occurrences.forEach((rawOccurrence, index) => {
    const parent = (rawOccurrence as SemanticOccurrence).parent
    if (parent !== null) runtimeChildren[parent].push(index)
  })
  const validationIndex: SemanticOccurrenceValidationIndex = {
    operationChildren,
    runtimeChildren,
    activationByDefinition: new Map(),
    proofBudget,
  }
  for (const definitionOccurrence of reparentedStaticRoots) {
    if (!moduleActivationAnchor(definitionOccurrence, occurrences, operations, validationIndex)) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${definitionOccurrence}].parent`, 'module-definition activation is not anchored to its exact matching call body')
    }
  }
  for (const continuation of pendingContinuations) {
    if (!isChildrenExpansionContinuation(
      continuation.occurrence,
      continuation.operation,
      continuation.parent,
      continuation.staticParent,
      continuation.slots,
      occurrences,
      operations,
      validationIndex,
    )) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${continuation.occurrence}].parent`, '$expansion must be the exact children() continuation of its matching caller')
    }
  }
  occurrences.forEach((occurrence, index) => {
    for (const field of ['parent', 'staticParent'] as const) {
      const reference = (occurrence as Record<string, unknown>)[field] as number | null
      if (reference === null) continue
      const referencedId = (occurrences[reference] as SemanticOccurrence).occurrenceId
      if (occurrenceGroups.get(referencedId)?.[0] !== reference) {
        fail('E_SEMANTIC_ORDER', `$.core.occurrences[${index}].${field}`, 'occurrence references must target the canonical first row of a logical occurrence')
      }
    }
  })
  for (const indexes of occurrenceGroups.values()) {
    const outputRows = indexes.filter(index => (occurrences[index] as Record<string, unknown>).outputOrdinal !== null)
    if (indexes.length > 1 && outputRows.length !== indexes.length) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${indexes[1]}]`, 'an occurrence ID may repeat only for its output slots')
    }
    outputRows.forEach((index, ordinal) => {
      if ((occurrences[index] as Record<string, unknown>).outputOrdinal !== ordinal) {
        fail('E_SEMANTIC_ORDER', `$.core.occurrences[${index}].outputOrdinal`, 'output ordinals must be unique, ordered, and dense from zero per occurrence ID')
      }
    })
  }
  for (const ordinals of duplicateSlotGroups.values()) {
    const ordered = [...ordinals.keys()].sort((left, right) => left - right)
    if (ordered.some((ordinal, index) => ordinal !== index)) {
      fail('E_SEMANTIC_ORDER', '$.core.occurrences', 'duplicate ordinals for equal dynamic slot values must be dense from zero')
    }
  }
  return occurrences as unknown as readonly SemanticOccurrence[]
}

function occurrenceRoot(index: number, occurrences: readonly SemanticOccurrence[]): number {
  let current = index
  while (occurrences[current].parent !== null) current = occurrences[current].parent!
  return current
}

function occurrenceIsDescendantOf(
  candidate: number,
  ancestor: number,
  occurrences: readonly SemanticOccurrence[],
): boolean {
  let current: number | null = candidate
  while (current !== null) {
    if (current === ancestor) return true
    current = occurrences[current].parent
  }
  return false
}

function nodePreservesIdentity(root: number, target: number, nodes: readonly SemanticNode[]): boolean {
  let current = root
  while (true) {
    if (current === target) return true
    const node = nodes[current]
    if (node.kind !== 'transform') return false
    current = node.input
  }
}

function validateOutputRef(
  value: unknown,
  path: string,
  nodes: readonly SemanticNode[],
  occurrences: readonly SemanticOccurrence[],
  canonicalOccurrenceRows: readonly number[],
): number {
  exactKeys(value, ['node', 'producerOccurrence', 'identityOccurrence', 'color'], path)
  const node = integer(value.node, `${path}.node`, 0, Math.max(0, nodes.length - 1))
  const producerOccurrence = integer(value.producerOccurrence, `${path}.producerOccurrence`, 0, Math.max(0, occurrences.length - 1))
  const identityOccurrence = integer(value.identityOccurrence, `${path}.identityOccurrence`, 0, Math.max(0, occurrences.length - 1))
  if (nodes.length === 0 || occurrences.length === 0) fail('E_SEMANTIC_REFERENCE', path, 'output requires a node occurrence')
  if (occurrences[producerOccurrence].node !== node) fail('E_SEMANTIC_IDENTITY', path, 'output node must match its producer occurrence')
  if (occurrences[identityOccurrence].sceneEntityId === null) fail('E_SEMANTIC_IDENTITY', path, 'output identity occurrence must own a scene entity')
  const identityNode = occurrences[identityOccurrence].node
  const canonicalProducerOccurrence = canonicalOccurrenceRows[producerOccurrence]
  const identityIsInProducingBranch = occurrenceIsDescendantOf(
    identityOccurrence,
    producerOccurrence,
    occurrences,
  ) || (
    occurrences[canonicalProducerOccurrence].occurrenceId === occurrences[producerOccurrence].occurrenceId
    && occurrenceIsDescendantOf(identityOccurrence, canonicalProducerOccurrence, occurrences)
  )
  if (identityNode === null || !nodePreservesIdentity(node, identityNode, nodes)
    || occurrenceRoot(canonicalProducerOccurrence, occurrences) !== occurrenceRoot(identityOccurrence, occurrences)
    || !identityIsInProducingBranch) {
    fail('E_SEMANTIC_IDENTITY', `${path}.identityOccurrence`, 'output identity must belong to the producing evaluation branch and node lineage')
  }
  const color = tupleNumbers(value.color, 4, `${path}.color`)
  if (color.some(channel => channel < 0 || channel > 1)) fail('E_SEMANTIC_NUMBER', `${path}.color`, 'color channels must be in [0, 1]')
  return node
}

function validateResult(
  value: unknown,
  nodes: readonly SemanticNode[],
  occurrences: readonly SemanticOccurrence[],
): readonly number[] {
  const firstRowByOccurrenceId = new Map<string, number>()
  const canonicalOccurrenceRows = occurrences.map((occurrence, index) => {
    const first = firstRowByOccurrenceId.get(occurrence.occurrenceId)
    if (first !== undefined) return first
    firstRowByOccurrenceId.set(occurrence.occurrenceId, index)
    return index
  })
  if (!isRecord(value) || typeof value.tag !== 'string') fail('E_SEMANTIC_SCHEMA', '$.core.result', 'expected a discriminated result')
  const rootNodes: number[] = []
  const outputOccurrences = new Set<number>()
  let priorIdentityOccurrence = -1
  const recordIdentityOccurrence = (item: Record<string, unknown>, path: string) => {
    const occurrence = item.identityOccurrence as number
    if (occurrence <= priorIdentityOccurrence) {
      fail('E_SEMANTIC_ORDER', `${path}.identityOccurrence`, 'root outputs must follow language occurrence/output-slot order')
    }
    priorIdentityOccurrence = occurrence
    if (outputOccurrences.has(occurrence)) fail('E_SEMANTIC_IDENTITY', `${path}.identityOccurrence`, 'one occurrence cannot own multiple result entries')
    outputOccurrences.add(occurrence)
  }
  if (value.tag === 'empty') {
    exactKeys(value, ['tag', 'type'], '$.core.result')
    if (value.type !== 'never') fail('E_SEMANTIC_TYPE', '$.core.result.type', 'empty result has bottom type never')
  } else if (value.tag === 'single') {
    exactKeys(value, ['tag', 'item'], '$.core.result')
    const node = validateOutputRef(
      value.item, '$.core.result.item', nodes, occurrences, canonicalOccurrenceRows,
    )
    rootNodes.push(node)
    recordIdentityOccurrence(value.item as Record<string, unknown>, '$.core.result.item')
  } else if (value.tag === 'multi') {
    exactKeys(value, ['tag', 'items'], '$.core.result')
    const items = arrayValue(value.items, '$.core.result.items', SEMANTIC_PROGRAM_LIMITS.outputs)
    if (items.length < 2) fail('E_SEMANTIC_TYPE', '$.core.result.items', 'multi requires at least two ordered outputs')
    items.forEach((item, index) => {
      rootNodes.push(validateOutputRef(
        item,
        `$.core.result.items[${index}]`,
        nodes,
        occurrences,
        canonicalOccurrenceRows,
      ))
      recordIdentityOccurrence(item as Record<string, unknown>, `$.core.result.items[${index}]`)
    })
  } else {
    fail('E_SEMANTIC_FIELD', '$.core.result.tag', 'unknown result tag')
  }

  return rootNodes
}

interface ValidatedSemanticExecution {
  readonly evaluationOrder: readonly number[]
  readonly discardedEffects: readonly Readonly<{
    tag: 'legacy-difference-cutters'
    root: number
    ownerOccurrence: number
  }>[]
  readonly terminal: null | Readonly<{
    tag: 'legacy-language-error'
    occurrence: number
    diagnosticTemplate: number
    prefixFrontier: readonly Readonly<{ root: number; ownerOccurrence: number | null }>[]
  }>
}

function reachableNodes(nodes: readonly SemanticNode[], roots: readonly number[]): Uint8Array {
  const reached = new Uint8Array(nodes.length)
  const stack = [...roots]
  while (stack.length > 0) {
    const node = stack.pop()!
    if (reached[node]) continue
    reached[node] = 1
    stack.push(...semanticNodeInputs(nodes[node]))
  }
  return reached
}

function validateExecution(
  value: unknown,
  nodes: readonly SemanticNode[],
  occurrences: readonly SemanticOccurrence[],
  operations: readonly SemanticStaticOperation[],
  outputRoots: readonly number[],
  templates: readonly SemanticDiagnosticTemplate[],
  languageContract: 'legacy/current' | 'openscad-viewer/brep-1',
): ValidatedSemanticExecution {
  exactKeys(value, ['version', 'evaluationOrder', 'discardedEffects', 'terminal'], '$.core.execution')
  if (value.version !== SEMANTIC_PROGRAM_EXECUTION_VERSION) {
    fail('E_SEMANTIC_VERSION', '$.core.execution.version', 'unknown semantic execution contract')
  }
  const order = arrayValue(value.evaluationOrder, '$.core.execution.evaluationOrder', SEMANTIC_PROGRAM_LIMITS.nodes)
  if (order.length !== nodes.length) {
    fail('E_SEMANTIC_DAG', '$.core.execution.evaluationOrder', 'evaluation order must contain every node exactly once')
  }
  const position = new Int32Array(nodes.length)
  position.fill(-1)
  order.forEach((rawNode, index) => {
    const node = integer(rawNode, `$.core.execution.evaluationOrder[${index}]`, 0, Math.max(0, nodes.length - 1))
    if (nodes.length === 0) fail('E_SEMANTIC_REFERENCE', `$.core.execution.evaluationOrder[${index}]`, 'evaluation order references absent node')
    if (position[node] !== -1) fail('E_SEMANTIC_ORDER', `$.core.execution.evaluationOrder[${index}]`, 'evaluation order contains a duplicate node')
    for (const input of semanticNodeInputs(nodes[node])) {
      if (position[input] === -1) fail('E_SEMANTIC_DAG', `$.core.execution.evaluationOrder[${index}]`, 'a node is scheduled before one of its inputs')
    }
    position[node] = index
    if (node !== index) {
      fail('E_SEMANTIC_ORDER', `$.core.execution.evaluationOrder[${index}]`, 'v1.2 node storage order is the exact authored kernel evaluation order')
    }
  })

  const firstRowByOccurrenceId = new Map<string, number>()
  occurrences.forEach((occurrence, index) => {
    if (!firstRowByOccurrenceId.has(occurrence.occurrenceId)) firstRowByOccurrenceId.set(occurrence.occurrenceId, index)
  })
  const effects = arrayValue(
    value.discardedEffects,
    '$.core.execution.discardedEffects',
    SEMANTIC_PROGRAM_LIMITS.discardedEffects,
  )
  let priorEffectPosition = -1
  const effectRoots = new Set<number>()
  const validatedEffects = effects.map((effect, index) => {
    const path = `$.core.execution.discardedEffects[${index}]`
    exactKeys(effect, ['tag', 'root', 'ownerOccurrence'], path)
    if (effect.tag !== 'legacy-difference-cutters') fail('E_SEMANTIC_FIELD', `${path}.tag`, 'unknown discarded effect kind')
    if (languageContract !== 'legacy/current') fail('E_SEMANTIC_TYPE', path, 'discarded legacy effects are forbidden for brep-1')
    const root = integer(effect.root, `${path}.root`, 0, Math.max(0, nodes.length - 1))
    if (nodes.length === 0 || position[root] < 0) fail('E_SEMANTIC_REFERENCE', `${path}.root`, 'effect root is absent from evaluation order')
    if (effectRoots.has(root)) fail('E_SEMANTIC_ORDER', `${path}.root`, 'effect roots must be unique')
    if (position[root] <= priorEffectPosition) fail('E_SEMANTIC_ORDER', `${path}.root`, 'effects must follow kernel evaluation order')
    effectRoots.add(root)
    priorEffectPosition = position[root]
    const ownerOccurrence = integer(effect.ownerOccurrence, `${path}.ownerOccurrence`, 0, Math.max(0, occurrences.length - 1))
    if (occurrences.length === 0) fail('E_SEMANTIC_REFERENCE', `${path}.ownerOccurrence`, 'effect owner occurrence is absent')
    const owner = occurrences[ownerOccurrence]
    if (firstRowByOccurrenceId.get(owner.occurrenceId) !== ownerOccurrence) {
      fail('E_SEMANTIC_ORDER', `${path}.ownerOccurrence`, 'effect owner must use the canonical occurrence row')
    }
    return { tag: effect.tag, root, ownerOccurrence } as const
  })

  let terminal: ValidatedSemanticExecution['terminal'] = null
  if (value.terminal !== null) {
    const path = '$.core.execution.terminal'
    exactKeys(value.terminal, ['tag', 'occurrence', 'diagnosticTemplate', 'prefixFrontier'], path)
    if (value.terminal.tag !== 'legacy-language-error') fail('E_SEMANTIC_FIELD', `${path}.tag`, 'unknown semantic terminal')
    if (languageContract !== 'legacy/current') fail('E_SEMANTIC_TYPE', path, 'legacy terminal traces are forbidden for brep-1')
    if (outputRoots.length !== 0) fail('E_SEMANTIC_TYPE', '$.core.result', 'a terminal plan cannot publish geometry outputs')
    const diagnosticTemplate = integer(
      value.terminal.diagnosticTemplate,
      `${path}.diagnosticTemplate`,
      0,
      Math.max(0, templates.length - 1),
    )
    if (templates.length === 0 || templates[diagnosticTemplate].severity !== 'error') {
      fail('E_SEMANTIC_REFERENCE', `${path}.diagnosticTemplate`, 'terminal must reference an error diagnostic template')
    }
    const occurrence = integer(
      value.terminal.occurrence,
      `${path}.occurrence`,
      0,
      Math.max(0, occurrences.length - 1),
    )
    if (occurrences.length === 0 || firstRowByOccurrenceId.get(occurrences[occurrence].occurrenceId) !== occurrence) {
      fail('E_SEMANTIC_ORDER', `${path}.occurrence`, 'terminal must reference the canonical interrupted occurrence row')
    }
    const interruptedId = occurrences[occurrence].occurrenceId
    // A parent operation may fail after all of its children (and even after
    // materializing earlier map outputs), so "terminal is the final row" is
    // false. What the source-free graph can prove is that the claimed frame
    // remained active: every subsequently created row belongs to that logical
    // activation or its runtime subtree. Exact deepest-error provenance is a
    // trusted-lowerer property; structural SPE/SPC graphs are never executable.
    for (let index = occurrence + 1; index < occurrences.length; index++) {
      if (occurrences[index].occurrenceId === interruptedId
        || occurrenceIsDescendantOf(index, occurrence, occurrences)) continue
      fail(
        'E_SEMANTIC_ORDER',
        `${path}.occurrence`,
        'no later sibling activation may exist outside the interrupted terminal subtree',
      )
    }
    const template = templates[diagnosticTemplate]
    if (template.operation !== occurrences[occurrence].operation) {
      fail('E_SEMANTIC_IDENTITY', `${path}.diagnosticTemplate`, 'terminal template operation must equal the interrupted occurrence operation')
    }
    if (template.code !== 'LEGACY_LANGUAGE_ERROR'
      || template.arguments.length !== 2
      || template.arguments[0].name !== 'errorName'
      || template.arguments[0].value.tag !== 'string'
      || !['OpenSCADParseError', 'TypeError'].includes(template.arguments[0].value.value)
      || template.arguments[1].name !== 'detailSha256'
      || template.arguments[1].value.tag !== 'string'
      || !/^[a-f0-9]{64}$/.test(template.arguments[1].value.value)) {
      fail('E_SEMANTIC_IDENTITY', `${path}.diagnosticTemplate`, 'terminal requires the exact frozen legacy error template shape')
    }
    if (validatedEffects.length !== 0) {
      fail('E_SEMANTIC_FIELD', '$.core.execution.discardedEffects', 'terminal prefixes subsume earlier discarded work; separate effect rows are forbidden')
    }
    const frontier = arrayValue(value.terminal.prefixFrontier, `${path}.prefixFrontier`, SEMANTIC_PROGRAM_LIMITS.nodes)
    const frontierRoots = new Set<number>()
    const validatedFrontier = frontier.map((entry, index) => {
      const entryPath = `${path}.prefixFrontier[${index}]`
      exactKeys(entry, ['root', 'ownerOccurrence'], entryPath)
      const root = integer(entry.root, `${entryPath}.root`, 0, Math.max(0, nodes.length - 1))
      if (nodes.length === 0) fail('E_SEMANTIC_REFERENCE', `${entryPath}.root`, 'terminal prefix references absent node')
      if (frontierRoots.has(root)) fail('E_SEMANTIC_ORDER', `${entryPath}.root`, 'terminal prefix roots must be unique')
      frontierRoots.add(root)
      let ownerOccurrence: number | null = null
      if (entry.ownerOccurrence !== null) {
        ownerOccurrence = integer(entry.ownerOccurrence, `${entryPath}.ownerOccurrence`, 0, Math.max(0, occurrences.length - 1))
        if (occurrences.length === 0 || occurrences[ownerOccurrence].node !== root) {
          fail('E_SEMANTIC_IDENTITY', `${entryPath}.ownerOccurrence`, 'terminal prefix owner must materialize its root')
        }
      }
      const admittedProducers = new Set<number>()
      occurrences.forEach((candidate, candidateIndex) => {
        if (candidate.node !== root || candidate.outputOrdinal === null) return
        if (!semanticNodeProducerOperationNames(nodes[root]).includes(
          operations[candidate.operation].name,
        )) return
        const canonical = firstRowByOccurrenceId.get(candidate.occurrenceId)
        if (canonical !== undefined) admittedProducers.add(canonical)
      })
      if (admittedProducers.size === 1) {
        const exactOwner = [...admittedProducers][0]
        if (ownerOccurrence !== exactOwner) {
          fail('E_SEMANTIC_IDENTITY', `${entryPath}.ownerOccurrence`, 'terminal prefix root must name its exact materializer occurrence')
        }
      } else if (admittedProducers.size === 0
        && nodes[root].kind === 'boolean' && nodes[root].operation === 'union') {
        if (ownerOccurrence !== null) {
          fail('E_SEMANTIC_IDENTITY', `${entryPath}.ownerOccurrence`, 'internal union reducer must have a null terminal prefix owner')
        }
      } else {
        fail('E_SEMANTIC_IDENTITY', `${entryPath}.ownerOccurrence`, 'terminal prefix root has ambiguous or absent materializer evidence')
      }
      return { root, ownerOccurrence }
    })
    const consumed = new Uint8Array(nodes.length)
    nodes.forEach(node => semanticNodeInputs(node).forEach(input => { consumed[input] = 1 }))
    const maximal = order.filter(node => consumed[node as number] === 0) as number[]
    if (maximal.length !== validatedFrontier.length
      || maximal.some((root, index) => root !== validatedFrontier[index].root)) {
      fail('E_SEMANTIC_DAG', `${path}.prefixFrontier`, 'terminal prefix must list every maximal completed node in evaluation order')
    }
    terminal = { tag: value.terminal.tag, occurrence, diagnosticTemplate, prefixFrontier: validatedFrontier }
  } else if (templates.some(template => template.severity === 'error')) {
    fail('E_SEMANTIC_REFERENCE', '$.core.execution.terminal', 'error templates require the matching terminal plan')
  }

  const outputReachable = reachableNodes(nodes, outputRoots)
  for (const root of effectRoots) {
    if (outputReachable[root]) fail('E_SEMANTIC_DAG', '$.core.execution.discardedEffects', 'a discarded effect cannot be reachable from a published output')
  }
  const roots = terminal === null
    ? [...outputRoots, ...effectRoots]
    : terminal.prefixFrontier.map(entry => entry.root)
  const reached = reachableNodes(nodes, roots)
  if (reached.some(value => value === 0)) {
    fail('E_SEMANTIC_DAG', '$.core.nodes', 'every node must be reachable from a result, effect, or terminal prefix root')
  }
  return { evaluationOrder: order as number[], discardedEffects: validatedEffects, terminal }
}

type SemanticProductionRule =
  | 'primitive' | 'transform-map' | 'projection-map' | 'offset-map'
  | 'preserving-alias' | 'transparent' | 'boolean' | 'hull' | 'difference'
  | 'linear-extrude' | 'rotate-extrude' | 'terminal-unsupported'

function semanticProductionRule(
  operation: SemanticStaticOperation,
  allowInterruptedUnsupported = false,
): SemanticProductionRule {
  const pair = `${operation.category}:${operation.name}`
  const last = operation.structuralPath[operation.structuralPath.length - 1]
  const prior = operation.structuralPath[operation.structuralPath.length - 2]
  const authoredCall = last.kind === 'call'
  if (operation.category === 'geometry' && authoredCall && ['cube', 'sphere', 'cylinder', 'polyhedron', 'square', 'circle', 'polygon'].includes(operation.name)) return 'primitive'
  if (operation.category === 'transform' && authoredCall && ['translate', 'rotate', 'scale', 'mirror', 'multmatrix'].includes(operation.name)) return 'transform-map'
  if (pair === 'geometry:projection' && authoredCall) return 'projection-map'
  if (pair === 'geometry:offset' && authoredCall) return 'offset-map'
  if ((pair === 'presentation:color' || pair === 'assertion:assert') && authoredCall) return 'preserving-alias'
  if (operation.category === 'control') {
    if (operation.name === '$assign' && last.kind === 'control') return 'transparent'
    if (operation.name === '$body' && operation.parent !== null
      && last.kind === 'body' && last.ordinal === 0 && prior !== undefined
      && ['call', 'module', 'control'].includes(prior.kind)) return 'transparent'
    if ((operation.name === '$then' || operation.name === '$else')
      && operation.parent !== null && last.kind === 'branch' && last.ordinal === 0 && prior?.name === 'if' && prior.kind === 'control') return 'transparent'
    if (operation.name === '$expansion' && operation.parent !== null && last.kind === 'control'
      && last.ordinal === 0
      && (prior?.name === '$body' || (prior?.name === 'children' && prior.kind === 'control'))) {
      return 'transparent'
    }
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
  fail('E_SEMANTIC_IDENTITY', `$.core.operations[${operation.id}]`, `operation pair ${pair} has no frozen v1 production rule`)
}

interface SemanticProductionItem {
  readonly node: number
  readonly row: number
  readonly identityOwner: number
  readonly group: number
  readonly directChild: number
}

type SemanticScheduleEvent =
  | { readonly tag: 'group'; readonly group: number }
  | { readonly tag: 'node'; readonly node: number }

interface SemanticProductionGroup {
  readonly canonical: number
  readonly outputRows: readonly number[]
  readonly outputItems: readonly SemanticProductionItem[]
  readonly frontier: readonly SemanticProductionItem[]
  readonly terminalItems: readonly SemanticProductionItem[]
  readonly schedule: readonly SemanticScheduleEvent[]
  readonly transparent: boolean
}

function equalNumberList(left: readonly number[], right: readonly number[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index])
}

function validateOccurrenceProduction(
  operations: readonly SemanticStaticOperation[],
  occurrences: readonly SemanticOccurrence[],
  nodes: readonly SemanticNode[],
  result: SemanticProgramCoreV1['result'],
  discardedEffects: ValidatedSemanticExecution['discardedEffects'] = [],
  terminal: ValidatedSemanticExecution['terminal'] = null,
): void {
  const terminalPrefix = terminal?.prefixFrontier ?? null
  const indexesById = new Map<string, number[]>()
  occurrences.forEach((occurrence, index) => {
    const indexes = indexesById.get(occurrence.occurrenceId) ?? []
    indexes.push(index)
    indexesById.set(occurrence.occurrenceId, indexes)
  })
  const canonicalByRow = new Int32Array(occurrences.length)
  const canonicalGroups: number[] = []
  const rowsByCanonical = new Map<number, readonly number[]>()
  for (const indexes of indexesById.values()) {
    const canonical = indexes[0]
    canonicalGroups.push(canonical)
    rowsByCanonical.set(canonical, indexes)
    indexes.forEach(index => { canonicalByRow[index] = canonical })
    const rows = indexes.map(index => occurrences[index])
    const zero = rows.length === 1 && rows[0].node === null
      && rows[0].outputOrdinal === null && rows[0].sceneEntityId === null
    const output = rows.every((row, ordinal) => row.node !== null
      && row.outputOrdinal === ordinal && row.sceneEntityId !== null)
    if (!zero && !output) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'a logical occurrence must be exactly one zero/frame row or dense output rows')
    }
  }
  canonicalGroups.sort((left, right) => left - right)
  const activeGroups = new Set<number>()
  let interruptedGroup: number | null = null
  if (terminal !== null) {
    let cursor: number | null = terminal.occurrence
    interruptedGroup = canonicalByRow[cursor]
    while (cursor !== null) {
      const canonical: number = canonicalByRow[cursor]
      activeGroups.add(canonical)
      cursor = occurrences[canonical].parent
    }
  }
  const effectByOwner = new Map<number, ValidatedSemanticExecution['discardedEffects'][number]>()
  for (const effect of discardedEffects) {
    const owner = canonicalByRow[effect.ownerOccurrence]
    if (effectByOwner.has(owner)) {
      fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'one difference occurrence can own only one discarded cutter effect')
    }
    effectByOwner.set(owner, effect)
  }
  const consumedEffectOwners = new Set<number>()
  const terminalEffectRoots = new Set<number>()
  const children = new Map<number, number[]>()
  for (const canonical of canonicalGroups) {
    const parent = occurrences[canonical].parent
    if (parent === null) continue
    const parentCanonical = canonicalByRow[parent]
    const list = children.get(parentCanonical) ?? []
    list.push(canonical)
    children.set(parentCanonical, list)
  }

  const ownerByNode = new Map<number, number>()
  const productionByCanonical = new Map<number, SemanticProductionGroup>()
  let expandedFrontierItems = 0
  const appendFrontier = (
    target: SemanticProductionItem[],
    source: readonly SemanticProductionItem[],
    directChild?: number,
  ): void => {
    expandedFrontierItems += source.length
    if (expandedFrontierItems > SEMANTIC_PROGRAM_LIMITS.snapshotValues) {
      fail('E_SEMANTIC_LIMIT', '$.core.occurrences', 'expanded occurrence frontier exceeds its bounded proof budget')
    }
    for (const item of source) {
      target.push(directChild === undefined ? item : { ...item, directChild })
    }
  }
  const claimNode = (node: number, canonical: number) => {
    const existing = ownerByNode.get(node)
    if (existing !== undefined) {
      fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${node}]`, 'each DAG node must have exactly one recomputed materializer group')
    }
    ownerByNode.set(node, canonical)
  }
  const requireOutputCount = (canonical: number, rows: readonly number[], expected: number) => {
    if (rows.length !== expected) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, `production rule requires exactly ${expected} output row(s)`)
    }
  }
  const requireNodeKind = (row: number, expected: readonly SemanticNode['kind'][]): SemanticNode => {
    const reference = occurrences[row].node
    if (reference === null || !expected.includes(nodes[reference].kind)) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, `operation cannot materialize node kind ${reference === null ? 'null' : nodes[reference].kind}`)
    }
    return nodes[reference]
  }

  const evaluate = (canonical: number): SemanticProductionGroup => {
    const cached = productionByCanonical.get(canonical)
    if (cached) return cached
    const groupRows = rowsByCanonical.get(canonical)!
    const outputRows = groupRows.filter(row => occurrences[row].node !== null)
    const operation = operations[occurrences[canonical].operation]
    const active = activeGroups.has(canonical)
    const rule = semanticProductionRule(operation, canonical === interruptedGroup)
    const isCompilerFrontierFrame = operation.category === 'control'
      && ['$body', '$then', '$else', '$expansion'].includes(operation.name)
    const frontier: SemanticProductionItem[] = []
    let hasBody = false
    const schedule: SemanticScheduleEvent[] = []
    for (const child of children.get(canonical) ?? []) {
      const childProduction = productionByCanonical.get(child)
      if (!childProduction) fail('E_SEMANTIC_ORDER', `$.core.occurrences[${child}]`, 'runtime occurrence groups must follow parent-before-child order')
      const childEvent: SemanticScheduleEvent = { tag: 'group', group: child }
      schedule.push(childEvent)
      const visible = childProduction.terminalItems
      if (isCompilerFrontierFrame) {
        const bucket: SemanticProductionItem[] = []
        appendFrontier(bucket, visible, child)
        appendFrontier(frontier, bucket)
      } else {
        appendFrontier(frontier, visible)
      }
      const childOperation = operations[occurrences[child].operation]
      if (childOperation.category === 'control' && childOperation.name === '$body') {
        if (hasBody) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'operation has more than one compiler-owned body frontier')
        }
        hasBody = true
      }
    }
    const frontierNodes = new Set<number>()
    for (const item of frontier) {
      if (frontierNodes.has(item.node)) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'cross-branch or duplicate frontier node reuse is forbidden in v1')
      }
      frontierNodes.add(item.node)
    }
    const outputItems: SemanticProductionItem[] = []
    let terminalItems: SemanticProductionItem[] | null = null
    const scheduleNode = (node: number) => {
      claimNode(node, canonical)
      schedule.push({ tag: 'node', node })
    }
    const own = (row: number, node: number) => {
      scheduleNode(node)
      outputItems.push({ node, row, identityOwner: row, group: canonical, directChild: canonical })
    }
    const preserve = (row: number, item: SemanticProductionItem) => {
      outputItems.push({ node: item.node, row, identityOwner: item.identityOwner, group: canonical, directChild: canonical })
    }
    if (active && rule === 'difference' && outputRows.length === 0) {
      const directGroups = children.get(canonical) ?? []
      if ((hasBody && directGroups.length !== 1) || (!hasBody && directGroups.length !== 0)) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'interrupted difference has an invalid compiler-owned body')
      }
      const bodyGroup = hasBody ? directGroups[0] : null
      const bodyOperation = bodyGroup === null ? null : occurrences[bodyGroup].operation
      const authoredBucketOperations = bodyOperation === null
        ? []
        : operations.filter(candidate => candidate.parent === bodyOperation).map(candidate => candidate.id)
      const runtimeBucketRoots = bodyGroup === null ? [] : children.get(bodyGroup) ?? []
      const authoredBucketSet = new Set(authoredBucketOperations)
      if (runtimeBucketRoots.some(child => !authoredBucketSet.has(occurrences[child].operation))) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'active difference runtime activation is not owned by an authored body statement')
      }
      const activeBuckets = authoredBucketOperations.map(bucketOperation => {
        const bucket: SemanticProductionItem[] = []
        for (const child of runtimeBucketRoots) {
          if (occurrences[child].operation !== bucketOperation) continue
          appendFrontier(bucket, productionByCanonical.get(child)!.terminalItems, child)
        }
        return bucket
      })
      const activeBucketSchedules = authoredBucketOperations.map(bucketOperation => (
        runtimeBucketRoots
          .filter(child => occurrences[child].operation === bucketOperation)
          .map(child => ({ tag: 'group', group: child }) as const)
      ))
      const reconstructedFrontier: SemanticProductionItem[] = []
      for (const bucket of activeBuckets) appendFrontier(reconstructedFrontier, bucket)
      if (reconstructedFrontier.length !== frontier.length
        || reconstructedFrontier.some((item, index) => {
          const expected = frontier[index]
          return item.node !== expected.node || item.row !== expected.row
            || item.identityOwner !== expected.identityOwner
            || item.directChild !== expected.directChild
        })) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'authored active difference buckets must exactly partition its runtime frontier')
      }
      const rawBase = activeBuckets[0] ?? []
      const rawCutters: SemanticProductionItem[] = []
      for (let bucket = 1; bucket < activeBuckets.length; bucket++) appendFrontier(rawCutters, activeBuckets[bucket])
      const firstStaticStatement = authoredBucketOperations[0]
      const baseRuntimeStatements = firstStaticStatement === undefined
        ? []
        : runtimeBucketRoots.filter(child => occurrences[child].operation === firstStaticStatement)
      const baseActivationIsPresent = baseRuntimeStatements.length > 0
      const base = baseActivationIsPresent ? rawBase : []
      const cutters = baseActivationIsPresent ? rawCutters : frontier
      const baseCompleted = baseActivationIsPresent
        && baseRuntimeStatements.every(statement => !activeGroups.has(statement))
      const cutterStatements = runtimeBucketRoots.filter(statement => (
        firstStaticStatement === undefined || occurrences[statement].operation !== firstStaticStatement
      ))
      const cuttersCompleted = canonical === interruptedGroup
        && cutterStatements.every(statement => !activeGroups.has(statement))
      schedule.length = 0
      if (baseActivationIsPresent && activeBucketSchedules[0] !== undefined) {
        schedule.push(...activeBucketSchedules[0])
      }
      const completeReduction = (
        items: readonly SemanticProductionItem[],
      ): SemanticProductionItem[] => {
        if (items.length < 2) return [...items]
        const space = nodes[items[0].node].valueType.space
        if (items.some(item => nodes[item.node].valueType.space !== space)) return [...items]
        const candidates = nodes.filter(candidate => !ownerByNode.has(candidate.id)
          && candidate.kind === 'boolean' && candidate.operation === 'union'
          && equalNumberList(candidate.inputs, items.map(item => item.node)))
        if (candidates.length !== 1) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'completed interrupted partition requires one exact union reducer')
        }
        scheduleNode(candidates[0].id)
        return [{
          node: candidates[0].id,
          row: items[0].row,
          identityOwner: items[0].identityOwner,
          group: canonical,
          directChild: canonical,
        }]
      }
      const reducedBase = baseCompleted ? completeReduction(base) : [...base]
      const firstCutterBucket = baseActivationIsPresent ? 1 : 0
      for (let bucket = firstCutterBucket; bucket < activeBucketSchedules.length; bucket++) {
        schedule.push(...activeBucketSchedules[bucket])
      }
      const reducedCutters = cuttersCompleted ? completeReduction(cutters) : [...cutters]
      terminalItems = [...reducedBase, ...reducedCutters]
    } else if (active && (rule === 'transform-map' || rule === 'projection-map' || rule === 'offset-map')) {
      if (outputRows.length > frontier.length) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'partial map emits beyond its completed frontier')
      }
      const commonParameters = new Map<string, string>()
      outputRows.forEach((row, index) => {
        const expectedKind = rule === 'transform-map' ? ['transform'] as const
          : rule === 'projection-map' ? ['projection'] as const : ['offset'] as const
        const produced = requireNodeKind(row, expectedKind)
        if (!equalNumberList(semanticNodeInputs(produced), [frontier[index].node])) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'partial map output must consume the matching ordered frontier item')
        }
        const parameters = produced.kind === 'transform' ? JSON.stringify(produced.matrix)
          : produced.kind === 'projection' ? String(produced.cut)
            : produced.kind === 'offset' ? String(produced.distance) : ''
        const parameterBucket = rule === 'transform-map' && operation.name === 'rotate'
          ? nodes[frontier[index].node].valueType.space
          : 'all'
        const priorParameters = commonParameters.get(parameterBucket)
        if (priorParameters !== undefined && parameters !== priorParameters) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'partial map outputs must use field-equal common parameters')
        }
        commonParameters.set(parameterBucket, parameters)
        scheduleNode(produced.id)
        outputItems.push({
          node: produced.id,
          row,
          identityOwner: rule === 'transform-map' ? frontier[index].identityOwner : row,
          group: canonical,
          directChild: canonical,
        })
      })
      terminalItems = [...outputItems]
      appendFrontier(terminalItems, frontier.slice(outputRows.length))
    } else if (active && outputRows.length === 0) {
      // Only the active call chain may have an absent result. Completed
      // siblings still pass their full closed production rules below.
      terminalItems = [...frontier]
    } else if (rule === 'transparent') {
      requireOutputCount(canonical, outputRows, 0)
    } else if (rule === 'primitive') {
      if (frontier.length !== 0) fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'primitive production frontier must be empty')
      requireOutputCount(canonical, outputRows, 1)
      const expectedByName: Readonly<Record<string, readonly SemanticNode['kind'][]>> = {
        cube: ['box'], sphere: ['sphere-analytic', 'sphere-polygonal'],
        cylinder: ['cylinder-analytic', 'cylinder-polygonal'], polyhedron: ['polyhedron'],
        square: ['rectangle'], circle: ['circle-analytic', 'circle-polygonal'], polygon: ['polygon'],
      }
      const produced = requireNodeKind(outputRows[0], expectedByName[operation.name])
      own(outputRows[0], produced.id)
    } else if (rule === 'transform-map' || rule === 'projection-map' || rule === 'offset-map') {
      requireOutputCount(canonical, outputRows, frontier.length)
      const commonParameters = new Map<string, string>()
      outputRows.forEach((row, index) => {
        const expectedKind = rule === 'transform-map' ? ['transform'] as const
          : rule === 'projection-map' ? ['projection'] as const : ['offset'] as const
        const produced = requireNodeKind(row, expectedKind)
        const input = semanticNodeInputs(produced)
        if (input.length !== 1 || input[0] !== frontier[index].node) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'map output must consume its matching ordered frontier item')
        }
        let parameters: string
        if (produced.kind === 'transform') parameters = JSON.stringify(produced.matrix)
        else if (produced.kind === 'projection') parameters = String(produced.cut)
        else if (produced.kind === 'offset') parameters = String(produced.distance)
        else fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, 'unexpected map node kind')
        const parameterBucket = rule === 'transform-map' && operation.name === 'rotate'
          ? nodes[frontier[index].node].valueType.space
          : 'all'
        const priorParameters = commonParameters.get(parameterBucket)
        if (priorParameters !== undefined && parameters !== priorParameters) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'one map occurrence must use field-equal common parameters')
        }
        commonParameters.set(parameterBucket, parameters)
        if (rule === 'transform-map') {
          scheduleNode(produced.id)
          outputItems.push({
            node: produced.id,
            row,
            identityOwner: frontier[index].identityOwner,
            group: canonical,
            directChild: canonical,
          })
        }
        else own(row, produced.id)
      })
    } else if (rule === 'preserving-alias') {
      requireOutputCount(canonical, outputRows, frontier.length)
      outputRows.forEach((row, index) => {
        if (occurrences[row].node !== frontier[index].node) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${row}].node`, 'alias output must equal its matching ordered frontier node')
        }
        preserve(row, frontier[index])
      })
    } else if (rule === 'boolean' || rule === 'hull') {
      if (frontier.length === 0) {
        requireOutputCount(canonical, outputRows, 0)
      } else if (frontier.length === 1) {
        requireOutputCount(canonical, outputRows, 1)
        if (occurrences[outputRows[0]].node !== frontier[0].node) {
          fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${outputRows[0]}].node`, 'one-item reduction must alias its sole frontier node')
        }
        if (rule === 'hull') outputItems.push({ node: frontier[0].node, row: outputRows[0], identityOwner: outputRows[0], group: canonical, directChild: canonical })
        else preserve(outputRows[0], frontier[0])
      } else {
        requireOutputCount(canonical, outputRows, 1)
        const expectedKind = rule === 'hull' ? ['hull'] as const : ['boolean'] as const
        const produced = requireNodeKind(outputRows[0], expectedKind)
        if (produced.kind === 'boolean') {
          if (produced.operation !== operation.name) {
            fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}].operation`, 'Boolean node operation does not match its static operation')
          }
        }
        if (!equalNumberList(semanticNodeInputs(produced), frontier.map(item => item.node))) {
          fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, 'n-ary reduction inputs must exactly equal the ordered occurrence frontier')
        }
        own(outputRows[0], produced.id)
      }
    } else if (rule === 'difference') {
      const directChildren = children.get(canonical) ?? []
      const bodies = directChildren.filter(child => {
        const childOperation = operations[occurrences[child].operation]
        const last = childOperation.structuralPath.at(-1)
        return childOperation.parent === occurrences[canonical].operation
          && childOperation.category === 'control'
          && childOperation.name === '$body'
          && last?.kind === 'body'
          && last.name === '$body'
      })
      if (bodies.length > 1 || (bodies.length === 1 && directChildren.length !== 1)
        || (bodies.length === 0 && directChildren.length !== 0)) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'difference runtime frontier requires at most one exact compiler $body child and no direct authored children')
      }
      const body = bodies[0]
      const bodyOperation = body === undefined ? null : occurrences[body].operation
      const authoredBucketOperations = bodyOperation === null
        ? []
        : operations.filter(candidate => candidate.parent === bodyOperation).map(candidate => candidate.id)
      const runtimeBucketRoots = body === undefined ? [] : children.get(body) ?? []
      const authoredBucketSet = new Set(authoredBucketOperations)
      if (runtimeBucketRoots.some(child => !authoredBucketSet.has(occurrences[child].operation))) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'difference runtime activation is not owned by a direct authored body statement')
      }
      const buckets = authoredBucketOperations.map(bucketOperation => {
        const bucket: SemanticProductionItem[] = []
        for (const child of runtimeBucketRoots) {
          if (occurrences[child].operation !== bucketOperation) continue
          const childProduction = productionByCanonical.get(child)!
          const visible = childProduction.outputItems.length > 0
            ? childProduction.outputItems
            : childProduction.transparent ? childProduction.frontier : []
          appendFrontier(bucket, visible, child)
        }
        return bucket
      })
      const bucketFrontier: SemanticProductionItem[] = []
      for (const bucket of buckets) appendFrontier(bucketFrontier, bucket)
      if (!equalNumberList(bucketFrontier.map(item => item.row), frontier.map(item => item.row))) {
        fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'difference frontier buckets must exactly partition its ordered frontier')
      }
      const base = buckets[0] ?? []
      const cutters: SemanticProductionItem[] = []
      for (let bucket = 1; bucket < buckets.length; bucket++) appendFrontier(cutters, buckets[bucket])
      const bucketSchedules = authoredBucketOperations.map(bucketOperation => {
        const bucketSchedule: SemanticScheduleEvent[] = []
        for (const child of runtimeBucketRoots) {
          if (occurrences[child].operation !== bucketOperation) continue
          bucketSchedule.push({ tag: 'group', group: child })
        }
        return bucketSchedule
      })
      let baseReducer: number | null = null
      let cutterReducer: number | null = null
      let differenceRoot: number | null = null
      const effect = effectByOwner.get(canonical)
      if (effect !== undefined) {
        requireOutputCount(canonical, outputRows, 0)
        if (base.length !== 0) {
          fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'discarded cutters require an exact empty first difference activation')
        }
        if (cutters.length === 0) {
          fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'empty cutters cannot create a discarded effect')
        }
        consumedEffectOwners.add(canonical)
        if (cutters.length === 1) {
          if (effect.root !== cutters[0].node) {
            fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'single cutter effect must preserve its exact frontier node')
          }
        } else {
          const reducer = nodes[effect.root]
          if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
            || !equalNumberList(reducer.inputs, cutters.map(item => item.node))) {
            fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'discarded cutters require one canonical ordered union reducer')
          }
          claimNode(reducer.id, canonical)
          cutterReducer = reducer.id
        }
      } else if (base.length === 0) {
        requireOutputCount(canonical, outputRows, 0)
        if (cutters.length > 0) {
          if (terminalPrefix === null) {
            fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'empty-base difference must retain its eager cutter effect')
          }
          const root = cutters.length === 1
            ? cutters[0].node
            : terminalPrefix.find(entry => {
              const candidate = nodes[entry.root]
              return candidate?.kind === 'boolean' && candidate.operation === 'union'
                && equalNumberList(candidate.inputs, cutters.map(item => item.node))
            })?.root
          if (root === undefined || !terminalPrefix.some(entry => entry.root === root)) {
            fail('E_SEMANTIC_IDENTITY', '$.core.execution.terminal.prefixFrontier', 'terminal prefix omits a completed eager difference cutter root')
          }
          if (cutters.length > 1) {
            claimNode(root, canonical)
            cutterReducer = root
          }
          consumedEffectOwners.add(canonical)
          terminalEffectRoots.add(root)
        }
      } else if (cutters.length === 0) {
        requireOutputCount(canonical, outputRows, 1)
        const outputNode = occurrences[outputRows[0]].node!
        if (base.length === 1) {
          if (outputNode !== base[0].node) fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${outputRows[0]}].node`, 'single-item difference base must alias its source node')
          preserve(outputRows[0], base[0])
        } else {
          const reducer = nodes[outputNode]
          if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
            || !equalNumberList(reducer.inputs, base.map(item => item.node))) {
            fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${outputRows[0]}].node`, 'multi-item difference base requires one canonical union reducer')
          }
          baseReducer = reducer.id
          own(outputRows[0], reducer.id)
        }
      } else {
        requireOutputCount(canonical, outputRows, 1)
        const produced = requireNodeKind(outputRows[0], ['boolean'])
        if (produced.kind !== 'boolean' || produced.operation !== 'difference' || produced.inputs.length !== 2) {
          fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, 'difference root must consume exactly reduced base and cutters')
        }
        const requireReduction = (reference: number, items: readonly SemanticProductionItem[], label: string) => {
          if (items.length === 1) {
            if (reference !== items[0].node) fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, `${label} reduction must preserve its sole frontier node`)
            return
          }
          const reducer = nodes[reference]
          if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
            || !equalNumberList(reducer.inputs, items.map(item => item.node))) {
            fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}]`, `${label} requires one canonical ordered union reducer`)
          }
          claimNode(reducer.id, canonical)
          if (label === 'difference base') baseReducer = reducer.id
          else cutterReducer = reducer.id
        }
        requireReduction(produced.inputs[0], base, 'difference base')
        requireReduction(produced.inputs[1], cutters, 'difference cutters')
        differenceRoot = produced.id
        own(outputRows[0], produced.id)
      }
      // A difference is the one rule whose compiler-owned reductions are
      // interleaved with child evaluation: the base is reduced before any
      // cutter statement starts. Rebuild this group's authenticated schedule
      // from authored statement buckets instead of generic child postorder.
      schedule.length = 0
      schedule.push(...(bucketSchedules[0] ?? []))
      if (baseReducer !== null) schedule.push({ tag: 'node', node: baseReducer })
      for (let bucket = 1; bucket < bucketSchedules.length; bucket++) {
        schedule.push(...bucketSchedules[bucket])
      }
      if (cutterReducer !== null) schedule.push({ tag: 'node', node: cutterReducer })
      if (differenceRoot !== null) schedule.push({ tag: 'node', node: differenceRoot })
    } else {
      if (frontier.length === 0) {
        requireOutputCount(canonical, outputRows, 0)
      } else {
        requireOutputCount(canonical, outputRows, 1)
        const expectedKind = rule === 'linear-extrude'
          ? ['linear-extrude'] as const
          : ['rotate-extrude-analytic', 'rotate-extrude-polygonal'] as const
        const produced = requireNodeKind(outputRows[0], expectedKind)
        const directInput = semanticNodeInputs(produced)[0]
        if (frontier.length === 1) {
          if (directInput !== frontier[0].node) fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}].input`, 'extrusion must consume its sole profile frontier node')
        } else {
          const reducer = nodes[directInput]
          if (reducer?.kind !== 'boolean' || reducer.operation !== 'union'
            || !equalNumberList(reducer.inputs, frontier.map(item => item.node))) {
            fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${produced.id}].input`, 'multi-profile extrusion requires one canonical internal union reduction')
          }
          claimNode(reducer.id, canonical)
          schedule.push({ tag: 'node', node: reducer.id })
        }
        own(outputRows[0], produced.id)
      }
    }

    if (terminalItems === null) {
      terminalItems = outputItems.length > 0
        ? [...outputItems]
        : rule === 'transparent' ? [...frontier] : []
    }
    const production = Object.freeze({
      canonical,
      outputRows: Object.freeze([...outputRows]),
      outputItems: Object.freeze(outputItems),
      frontier: Object.freeze(frontier),
      terminalItems: Object.freeze(terminalItems),
      schedule: Object.freeze(schedule),
      transparent: rule === 'transparent',
    })
    productionByCanonical.set(canonical, production)
    return production
  }

  for (let index = canonicalGroups.length - 1; index >= 0; index--) evaluate(canonicalGroups[index])
  if ([...effectByOwner.keys()].some(owner => !consumedEffectOwners.has(owner))) {
    fail('E_SEMANTIC_IDENTITY', '$.core.execution.discardedEffects', 'discarded effect is not owned by an exact empty-base difference occurrence')
  }
  if (ownerByNode.size !== nodes.length) {
    const unowned = nodes.find(node => !ownerByNode.has(node.id))
    fail('E_SEMANTIC_IDENTITY', unowned ? `$.core.nodes[${unowned.id}]` : '$.core.nodes', 'every reachable DAG node needs exactly one recomputed materializer group')
  }

  const scheduleStack: SemanticScheduleEvent[] = []
  for (let index = canonicalGroups.length - 1; index >= 0; index--) {
    const canonical = canonicalGroups[index]
    if (occurrences[canonical].parent === null) {
      scheduleStack.push({ tag: 'group', group: canonical })
    }
  }
  const authenticatedSchedule: number[] = []
  let scheduleEvents = 0
  while (scheduleStack.length > 0) {
    if (++scheduleEvents > SEMANTIC_PROGRAM_LIMITS.occurrences + SEMANTIC_PROGRAM_LIMITS.nodes * 2) {
      fail('E_SEMANTIC_LIMIT', '$.core.execution.evaluationOrder', 'occurrence-production schedule exceeds its bounded replay budget')
    }
    const event = scheduleStack.pop()!
    if (event.tag === 'node') {
      authenticatedSchedule.push(event.node)
      if (authenticatedSchedule.length > nodes.length) {
        fail('E_SEMANTIC_ORDER', '$.core.execution.evaluationOrder', 'occurrence-production schedule creates a node more than once')
      }
      continue
    }
    const childSchedule = productionByCanonical.get(event.group)?.schedule
    if (childSchedule === undefined) {
      fail('E_SEMANTIC_REFERENCE', '$.core.execution.evaluationOrder', 'occurrence-production schedule references an absent group')
    }
    for (let index = childSchedule.length - 1; index >= 0; index--) {
      scheduleStack.push(childSchedule[index])
    }
  }
  if (authenticatedSchedule.length !== nodes.length
    || authenticatedSchedule.some((node, index) => node !== index)) {
    fail('E_SEMANTIC_ORDER', '$.core.execution.evaluationOrder', 'node IDs must equal the exact occurrence-production schedule')
  }

  const programFrontier: SemanticProductionItem[] = []
  for (const canonical of canonicalGroups) {
    if (occurrences[canonical].parent !== null) continue
    const production = productionByCanonical.get(canonical)!
    appendFrontier(programFrontier, production.terminalItems)
  }
  const rootOutputs = semanticResultItems(result)
  const reachableOutputGroups = new Set<number>()
  const rootGroups: number[] = []
  if (terminalPrefix === null) {
    if (rootOutputs.length !== programFrontier.length) {
      fail('E_SEMANTIC_IDENTITY', '$.core.result', 'root result must exactly equal the synthetic program frontier')
    }
    rootOutputs.forEach((output, index) => {
      const slot = programFrontier[index]
      if (slot.row !== output.producerOccurrence || slot.node !== output.node) {
        fail('E_SEMANTIC_IDENTITY', '$.core.result', 'root producer must be an exact output row derived by its occurrence production rule')
      }
      if (slot.identityOwner !== output.identityOccurrence) {
        fail('E_SEMANTIC_IDENTITY', '$.core.result', 'root identity occurrence does not match the recursively derived production owner')
      }
      rootGroups.push(slot.group)
    })
  } else {
    if (rootOutputs.length !== 0) {
      fail('E_SEMANTIC_IDENTITY', '$.core.result', 'terminal result must remain bottom')
    }
    const derivedRoots = [...new Set([
      ...programFrontier.map(item => item.node),
      ...terminalEffectRoots,
    ])].sort((left, right) => left - right)
    if (derivedRoots.length !== terminalPrefix.length
      || derivedRoots.some((root, index) => root !== terminalPrefix[index].root)) {
      fail('E_SEMANTIC_IDENTITY', '$.core.execution.terminal.prefixFrontier', 'terminal prefix must exactly equal the completed production frontier')
    }
    for (const slot of programFrontier) rootGroups.push(slot.group)
  }
  for (const owner of consumedEffectOwners) rootGroups.push(owner)
  while (rootGroups.length > 0) {
    const canonical = rootGroups.pop()!
    if (reachableOutputGroups.has(canonical)) continue
    reachableOutputGroups.add(canonical)
    for (const item of productionByCanonical.get(canonical)!.frontier) rootGroups.push(item.group)
  }
  for (const canonical of canonicalGroups) {
    const production = productionByCanonical.get(canonical)!
    if (production.outputRows.length > 0 && !reachableOutputGroups.has(canonical)) {
      fail('E_SEMANTIC_IDENTITY', `$.core.occurrences[${canonical}]`, 'node-bearing occurrence group is not consumed by a root production path')
    }
  }
}

function validateDiagnosticTemplates(
  value: unknown,
  operations: readonly SemanticStaticOperation[],
): readonly SemanticDiagnosticTemplate[] {
  const templates = arrayValue(value, '$.core.diagnosticTemplates', SEMANTIC_PROGRAM_LIMITS.diagnostics)
  templates.forEach((template, index) => {
    const path = `$.core.diagnosticTemplates[${index}]`
    exactKeys(template, ['id', 'code', 'severity', 'operation', 'arguments'], path)
    if (template.id !== index) fail('E_SEMANTIC_ORDER', `${path}.id`, 'diagnostic template IDs must equal array positions')
    const code = stringValue(template.code, `${path}.code`, 64)
    if (!/^[A-Z][A-Z0-9_]{0,63}$/.test(code)) fail('E_SEMANTIC_FIELD', `${path}.code`, 'invalid diagnostic code')
    if (template.severity !== 'info' && template.severity !== 'warning' && template.severity !== 'error') {
      fail('E_SEMANTIC_FIELD', `${path}.severity`, 'unknown diagnostic severity')
    }
    if (template.severity === 'error' && code !== 'LEGACY_LANGUAGE_ERROR') {
      fail('E_SEMANTIC_FIELD', `${path}.code`, 'v1.2 admits only the frozen legacy terminal error template')
    }
    if (template.operation !== null) {
      integer(template.operation, `${path}.operation`, 0, Math.max(0, operations.length - 1))
      if (operations.length === 0) fail('E_SEMANTIC_REFERENCE', `${path}.operation`, 'diagnostic references absent operation')
    }
    const args = arrayValue(template.arguments, `${path}.arguments`, 128)
    const names = new Set<string>()
    args.forEach((argument, argumentIndex) => {
      const argumentPath = `${path}.arguments[${argumentIndex}]`
      exactKeys(argument, ['name', 'value'], argumentPath)
      const name = stringValue(argument.name, `${argumentPath}.name`, 128)
      if (names.has(name)) fail('E_SEMANTIC_ORDER', `${argumentPath}.name`, 'diagnostic argument names must be unique in authored order')
      names.add(name)
      validateIdentityValue(argument.value, `${argumentPath}.value`)
    })
  })
  return templates as unknown as readonly SemanticDiagnosticTemplate[]
}

function validateCore(value: unknown): asserts value is SemanticProgramCoreV1 {
  exactKeys(value, [
    'schema', 'schemaVersion', 'requiredFeatures', 'identityVersion', 'language',
    'units', 'operations', 'occurrences', 'nodes', 'execution', 'result', 'declaredCapabilities',
    'capabilityClosure', 'diagnosticTemplates',
  ], '$.core')
  if (value.schema !== 'semantic-program-core') fail('E_SEMANTIC_SCHEMA', '$.core.schema', 'expected semantic-program-core')
  assertSchemaVersion(value.schemaVersion, '$.core.schemaVersion')
  const features = validateSortedStrings(value.requiredFeatures, '$.core.requiredFeatures', 64, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  if (features.length !== 1 || features[0] !== SEMANTIC_PROGRAM_EXECUTION_FEATURE) {
    fail('E_SEMANTIC_VERSION', '$.core.requiredFeatures', 'v1.2 requires exactly semantic.execution-v2')
  }
  if (value.identityVersion !== SEMANTIC_PROGRAM_IDENTITY) fail('E_SEMANTIC_VERSION', '$.core.identityVersion', 'unknown core identity version')
  exactKeys(value.language, ['contract', 'semanticsRevision', 'capabilityGraphVersion'], '$.core.language')
  if (value.language.contract !== 'legacy/current' && value.language.contract !== 'openscad-viewer/brep-1') fail('E_SEMANTIC_FIELD', '$.core.language.contract', 'unknown language contract')
  const expectedRevision = value.language.contract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0'
  if (value.language.semanticsRevision !== expectedRevision) fail('E_SEMANTIC_VERSION', '$.core.language.semanticsRevision', `expected ${expectedRevision}`)
  if (value.language.capabilityGraphVersion !== SEMANTIC_PROGRAM_CAPABILITY_GRAPH) fail('E_SEMANTIC_VERSION', '$.core.language.capabilityGraphVersion', 'unknown capability graph')
  exactKeys(value.units, ['length', 'angle', 'handedness', 'upAxis', 'matrixLayout', 'composition'], '$.core.units')
  if (value.units.length !== 'millimeter' || value.units.angle !== 'degree'
    || value.units.handedness !== 'right' || value.units.upAxis !== 'z'
    || value.units.matrixLayout !== 'column-major' || value.units.composition !== 'parent-times-local') {
    fail('E_SEMANTIC_TYPE', '$.core.units', 'v1 units/frame/matrix convention is fixed')
  }
  const operations = validateOperations(value.operations)
  const nodes = validateNodes(value.nodes, value.language.contract)
  const occurrences = validateOccurrences(value.occurrences, operations, nodes)
  const templates = validateDiagnosticTemplates(value.diagnosticTemplates, operations)
  const outputRoots = validateResult(value.result, nodes, occurrences)
  const execution = validateExecution(
    value.execution,
    nodes,
    occurrences,
    operations,
    outputRoots,
    templates,
    value.language.contract,
  )
  validateOccurrenceProduction(
    operations,
    occurrences,
    nodes,
    value.result as SemanticProgramCoreV1['result'],
    execution.discardedEffects,
    execution.terminal,
  )
  const declared = validateSortedStrings(value.declaredCapabilities, '$.core.declaredCapabilities', SEMANTIC_PROGRAM_LIMITS.declaredCapabilities, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  const closure = validateSortedStrings(value.capabilityClosure, '$.core.capabilityClosure', SEMANTIC_PROGRAM_LIMITS.capabilityClosure, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  const expectedClosure = deriveSemanticCapabilityClosure({
    language: value.language as unknown as SemanticProgramCoreV1['language'],
    nodes,
    result: value.result as SemanticProgramCoreV1['result'],
    operations,
    occurrences,
    declaredCapabilities: declared,
    diagnosticTemplates: templates,
  })
  if (closure.length !== expectedClosure.length || closure.some((capability, index) => capability !== expectedClosure[index])) {
    fail('E_SEMANTIC_CAPABILITY_CLOSURE', '$.core.capabilityClosure', 'closure does not exactly match authored and inferred capabilities')
  }
}

function validateSpan(value: unknown, path: string, sourceLength: number, emptyAllowed: boolean): asserts value is SemanticSourceSpan {
  exactKeys(value, ['start', 'end'], path)
  const start = integer(value.start, `${path}.start`, 0, sourceLength)
  const end = integer(value.end, `${path}.end`, 0, sourceLength)
  if (end < start || (!emptyAllowed && end === start)) fail('E_SEMANTIC_PROVENANCE', path, 'invalid half-open UTF-16 source span')
}

export function validateSemanticProgramV1(value: unknown): asserts value is SemanticProgramEnvelopeV1 {
  exactKeys(value, ['schema', 'schemaVersion', 'source', 'core', 'provenance', 'tessellationIntents', 'diagnostics'], '$')
  if (value.schema !== 'semantic-program-envelope') fail('E_SEMANTIC_SCHEMA', '$.schema', 'expected semantic-program-envelope')
  assertSchemaVersion(value.schemaVersion, '$.schemaVersion')
  exactKeys(value.source, ['sha256', 'utf8ByteLength', 'utf16CodeUnitLength'], '$.source')
  if (typeof value.source.sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(value.source.sha256)) fail('E_SEMANTIC_FIELD', '$.source.sha256', 'expected lowercase SHA-256')
  integer(value.source.utf8ByteLength, '$.source.utf8ByteLength', 0, 4_000_000)
  const sourceLength = integer(value.source.utf16CodeUnitLength, '$.source.utf16CodeUnitLength', 0, 250_000)
  const core = value.core
  validateCore(core)

  const provenance = arrayValue(value.provenance, '$.provenance', SEMANTIC_PROGRAM_LIMITS.operations)
  if (provenance.length !== core.operations.length) fail('E_SEMANTIC_PROVENANCE', '$.provenance', 'every static operation needs exactly one source record')
  provenance.forEach((record, index) => {
    const path = `$.provenance[${index}]`
    exactKeys(record, ['operation', 'span', 'label'], path)
    if (record.operation !== index) fail('E_SEMANTIC_ORDER', `${path}.operation`, 'provenance must follow operation order')
    validateSpan(record.span, `${path}.span`, sourceLength, false)
    stringValue(record.label, `${path}.label`, 512)
  })

  const intents = arrayValue(value.tessellationIntents, '$.tessellationIntents', SEMANTIC_PROGRAM_LIMITS.tessellationIntents)
  if (core.language.contract === 'legacy/current' && intents.length !== 0) {
    fail('E_SEMANTIC_TYPE', '$.tessellationIntents', 'legacy/current materializes tessellation in SPC1 and cannot carry TSP1 intents')
  }
  let priorOccurrence = -1
  intents.forEach((intent, index) => {
    const path = `$.tessellationIntents[${index}]`
    exactKeys(intent, ['occurrence', 'chordTolerance', 'angularToleranceDegrees', 'minSegments', 'maxSegments'], path)
    const occurrence = integer(intent.occurrence, `${path}.occurrence`, 0, Math.max(0, core.occurrences.length - 1))
    if (core.occurrences.length === 0 || core.occurrences[occurrence].node === null) fail('E_SEMANTIC_REFERENCE', `${path}.occurrence`, 'tessellation intent needs a producing occurrence')
    if (occurrence <= priorOccurrence) fail('E_SEMANTIC_ORDER', `${path}.occurrence`, 'intents must be unique and ordered by occurrence')
    priorOccurrence = occurrence
    nullablePositive(intent.chordTolerance, `${path}.chordTolerance`)
    nullablePositive(intent.angularToleranceDegrees, `${path}.angularToleranceDegrees`)
    const minimum = intent.minSegments === null ? null : integer(intent.minSegments, `${path}.minSegments`, 3, 1_000_000)
    const maximum = intent.maxSegments === null ? null : integer(intent.maxSegments, `${path}.maxSegments`, 3, 1_000_000)
    if (minimum !== null && maximum !== null && maximum < minimum) fail('E_SEMANTIC_NUMBER', path, 'maxSegments cannot be below minSegments')
  })

  const diagnostics = arrayValue(value.diagnostics, '$.diagnostics', SEMANTIC_PROGRAM_LIMITS.diagnostics)
  if (diagnostics.length !== core.diagnosticTemplates.length) fail('E_SEMANTIC_PROVENANCE', '$.diagnostics', 'every diagnostic template needs exactly one source-bound presentation')
  diagnostics.forEach((diagnostic, index) => {
    const path = `$.diagnostics[${index}]`
    exactKeys(diagnostic, ['template', 'message', 'span'], path)
    if (diagnostic.template !== index) fail('E_SEMANTIC_ORDER', `${path}.template`, 'diagnostics must follow template order')
    stringValue(diagnostic.message, `${path}.message`, SEMANTIC_PROGRAM_LIMITS.diagnosticMessageCodeUnits)
    if (diagnostic.span !== null) validateSpan(diagnostic.span, `${path}.span`, sourceLength, true)
  })
}

interface SnapshotBudget { values: number }

function snapshotJsonValue(value: unknown, path: string, depth: number, ancestors: Set<object>, budget: SnapshotBudget): unknown {
  if (++budget.values > SEMANTIC_PROGRAM_LIMITS.snapshotValues) fail('E_SEMANTIC_LIMIT', path, 'snapshot value budget exceeded')
  if (depth > SEMANTIC_PROGRAM_LIMITS.snapshotDepth) fail('E_SEMANTIC_LIMIT', path, 'snapshot depth exceeded')
  if (value === null || typeof value === 'boolean') return value
  if (typeof value === 'number') return finiteNumber(value, path)
  if (typeof value === 'string') return stringValue(value, path, SEMANTIC_PROGRAM_LIMITS.stringCodeUnits)
  if (typeof value !== 'object') fail('E_SEMANTIC_SCHEMA', path, 'only JSON data is allowed')
  if (Object.getOwnPropertySymbols(value).length !== 0) fail('E_SEMANTIC_FIELD', path, 'symbol fields are forbidden')
  if (ancestors.has(value)) fail('E_SEMANTIC_DAG', path, 'wire object graph cannot be cyclic')
  ancestors.add(value)
  try {
    if (Array.isArray(value)) {
      const keys = Reflect.ownKeys(value)
      if (keys.length !== value.length + 1 || keys[keys.length - 1] !== 'length'
        || keys.slice(0, -1).some((key, index) => key !== String(index))) {
        fail('E_SEMANTIC_FIELD', path, 'arrays must be dense without symbol, hidden, or extra fields')
      }
      return Array.from({ length: value.length }, (_, index) => {
        const descriptor = Object.getOwnPropertyDescriptor(value, String(index))
        if (!descriptor || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) fail('E_SEMANTIC_FIELD', `${path}[${index}]`, 'array entries must be enumerable data properties')
        return snapshotJsonValue(descriptor.value, `${path}[${index}]`, depth + 1, ancestors, budget)
      })
    }
    const prototype = Object.getPrototypeOf(value)
    if (prototype !== Object.prototype && prototype !== null) fail('E_SEMANTIC_SCHEMA', path, 'only plain JSON objects are allowed')
    const output: Record<string, unknown> = Object.create(null)
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key !== 'string') fail('E_SEMANTIC_FIELD', path, 'symbol fields are forbidden')
      if (!wellFormedUnicode(key)) fail('E_SEMANTIC_FIELD', path, 'object keys must be well-formed Unicode')
      const descriptor = Object.getOwnPropertyDescriptor(value, key)
      if (!descriptor || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) fail('E_SEMANTIC_FIELD', `${path}.${key}`, 'object fields must be enumerable data properties')
      Object.defineProperty(output, key, {
        value: snapshotJsonValue(descriptor.value, `${path}.${key}`, depth + 1, ancestors, budget),
        enumerable: true,
        configurable: true,
        writable: true,
      })
    }
    return output
  } finally {
    ancestors.delete(value)
  }
}

function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.values(value as Record<string, unknown>).forEach(deepFreeze)
    Object.freeze(value)
  }
  return value
}

function validateCanonicalStringBudget(value: unknown): void {
  const encoder = new TextEncoder()
  const pending: Array<{ value: unknown; path: string }> = [{ value, path: '$' }]
  let bytes = 0
  const add = (text: string, path: string) => {
    bytes += encoder.encode(text).length
    if (bytes > SEMANTIC_PROGRAM_LIMITS.canonicalStringBytes) {
      fail('E_SEMANTIC_LIMIT', path, `canonical string/key bytes exceed ${SEMANTIC_PROGRAM_LIMITS.canonicalStringBytes}`)
    }
  }
  while (pending.length > 0) {
    const current = pending.pop()!
    if (typeof current.value === 'string') {
      add(current.value, current.path)
    } else if (Array.isArray(current.value)) {
      for (let index = current.value.length - 1; index >= 0; index--) pending.push({ value: current.value[index], path: `${current.path}[${index}]` })
    } else if (isRecord(current.value)) {
      const entries = Object.entries(current.value)
      for (let index = entries.length - 1; index >= 0; index--) {
        const [key, child] = entries[index]
        add(key, `${current.path}.<key>`)
        pending.push({ value: child, path: `${current.path}.${key}` })
      }
    }
  }
}

export function normalizeSemanticProgram(input: unknown, exactSource?: string): SemanticProgramV1 {
  let snapshot = snapshotJsonValue(input, '$', 0, new Set(), { values: 0 })
  if (!isRecord(snapshot)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program envelope')
  if (snapshot.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(snapshot)
  validateSemanticProgramV1(snapshot)
  const program = deepFreeze(snapshot)
  if (exactSource !== undefined) attestSemanticProgramSource(program, exactSource)
  return program
}

/**
 * Validates and freezes the graph produced exclusively by the bounded binary
 * decoder. Unlike the public normalizer, this does not duplicate that already
 * isolated, null-prototype JSON graph. Callers with arbitrary objects must use
 * normalizeSemanticProgram so descriptors and prototypes are snapshotted.
 */
export function normalizeDecodedSemanticProgram(input: unknown): SemanticProgramV1 {
  if (!isSemanticProgramCodecOwnedGraph(input)) {
    fail('E_SEMANTIC_SCHEMA', '$', 'in-place normalization requires a codec-owned decoded graph')
  }
  if (!isRecord(input)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program envelope')
  // Old wire prototypes cannot synthesize v1.2 execution evidence.
  if (input.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(input)
  validateSemanticProgramV1(input)
  return deepFreeze(input as unknown as SemanticProgramV1)
}

export function normalizeSemanticProgramCore(input: unknown): SemanticProgramCoreV1 {
  const snapshot = snapshotJsonValue(input, '$.core', 0, new Set(), { values: 0 })
  validateCanonicalStringBudget(snapshot)
  validateCore(snapshot)
  return deepFreeze(snapshot as unknown as SemanticProgramCoreV1)
}

/** In-place counterpart for the graph freshly allocated by ByteReader only. */
export function normalizeDecodedSemanticProgramCore(input: unknown): SemanticProgramCoreV1 {
  if (!isSemanticProgramCodecOwnedGraph(input)) {
    fail('E_SEMANTIC_SCHEMA', '$.core', 'in-place normalization requires a codec-owned decoded graph')
  }
  validateCanonicalStringBudget(input)
  validateCore(input)
  return deepFreeze(input as unknown as SemanticProgramCoreV1)
}

export function normalizeSemanticProgramCoreInput(input: unknown): SemanticProgramCoreV1 {
  let snapshot = snapshotJsonValue(input, '$', 0, new Set(), { values: 0 })
  if (!isRecord(snapshot)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program core or envelope')
  if (snapshot.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(snapshot)
  if (snapshot.schema === 'semantic-program-core') {
    validateCore(snapshot)
    return deepFreeze(snapshot as unknown as SemanticProgramCoreV1)
  }
  validateSemanticProgramV1(snapshot)
  return deepFreeze(snapshot as unknown as SemanticProgramV1).core
}

export function semanticSourceDescriptor(source: string): SemanticSourceDescriptor {
  if (!wellFormedUnicode(source)) fail('E_SEMANTIC_BYTES', '$source', 'semantic source must be well-formed Unicode before UTF-8 encoding')
  if (source.length > SEMANTIC_PROGRAM_LIMITS.sourceCodeUnits) {
    fail('E_SEMANTIC_LIMIT', '$source', `semantic source exceeds ${SEMANTIC_PROGRAM_LIMITS.sourceCodeUnits} UTF-16 code units`)
  }
  const bytes = new TextEncoder().encode(source)
  if (bytes.length > 4_000_000) fail('E_SEMANTIC_LIMIT', '$source', 'semantic source exceeds 4000000 UTF-8 bytes')
  return Object.freeze({
    sha256: sha256Hex(bytes),
    utf8ByteLength: bytes.length,
    utf16CodeUnitLength: source.length,
  })
}

export function attestSemanticProgramSource(program: SemanticProgramV1, source: string): void {
  const actual = semanticSourceDescriptor(source)
  if (actual.sha256 !== program.source.sha256
    || actual.utf8ByteLength !== program.source.utf8ByteLength
    || actual.utf16CodeUnitLength !== program.source.utf16CodeUnitLength) {
    fail('E_SEMANTIC_PROVENANCE', '$.source', 'source bytes do not match the envelope attestation')
  }
  let route: ReturnType<typeof parseGeometrySourceRoutingHeader>
  try {
    route = parseGeometrySourceRoutingHeader(source)
  } catch (error) {
    fail('E_SEMANTIC_PROVENANCE', '$source', `source routing header is invalid: ${error instanceof Error ? error.message : String(error)}`)
  }
  if (route.languageContract !== program.core.language.contract) {
    fail('E_SEMANTIC_PROVENANCE', '$.core.language.contract', 'source routing language does not match the semantic core')
  }
  if (route.requiredCapabilities.length !== program.core.declaredCapabilities.length
    || route.requiredCapabilities.some((capability, index) => capability !== program.core.declaredCapabilities[index])) {
    fail('E_SEMANTIC_PROVENANCE', '$.core.declaredCapabilities', 'source routing requirements do not match authored semantic capabilities')
  }
  const assertScalarBoundary = (offset: number, path: string) => {
    if (offset > 0 && offset < source.length) {
      const prior = source.charCodeAt(offset - 1)
      const next = source.charCodeAt(offset)
      if (prior >= 0xd800 && prior <= 0xdbff && next >= 0xdc00 && next <= 0xdfff) {
        fail('E_SEMANTIC_PROVENANCE', path, 'UTF-16 span endpoint splits a surrogate pair')
      }
    }
  }
  program.provenance.forEach((record, index) => {
    assertScalarBoundary(record.span.start, `$.provenance[${index}].span.start`)
    assertScalarBoundary(record.span.end, `$.provenance[${index}].span.end`)
  })
  program.diagnostics.forEach((diagnostic, index) => {
    if (diagnostic.span === null) return
    assertScalarBoundary(diagnostic.span.start, `$.diagnostics[${index}].span.start`)
    assertScalarBoundary(diagnostic.span.end, `$.diagnostics[${index}].span.end`)
  })
}

export function semanticDiagnosticCodeIsPublic(value: string): value is SemanticProgramDiagnosticCode {
  return (SEMANTIC_PROGRAM_DIAGNOSTIC_CODES as readonly string[]).includes(value)
}
