import {
  deriveSemanticAmbiguityGroupId,
  deriveSemanticOperationId,
  SEMANTIC_PROGRAM_LIMITS,
  type SemanticDiagnosticTemplate,
  type SemanticDynamicSlot,
  type SemanticIdentityValue,
  type SemanticNode,
  type SemanticOperationCategory,
  type SemanticProgramDiagnosticCode,
  type SemanticStaticOperation,
  type SemanticStructuralPathSegment,
  type SemanticValueType,
} from '../core/semanticProgram'

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

export function fail(code: SemanticProgramDiagnosticCode, path: string, detail: string): never {
  throw new SemanticProgramValidationError(code, path, detail)
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && !Array.isArray(value) && typeof value === 'object'
}

export function exactKeys(value: unknown, expected: readonly string[], path: string): asserts value is Record<string, unknown> {
  if (!isRecord(value)) fail('E_SEMANTIC_SCHEMA', path, 'expected an object')
  const actual = Object.keys(value).sort()
  const wanted = [...expected].sort()
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail('E_SEMANTIC_FIELD', path, 'unknown or missing fields')
  }
}

export function wellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

export function stringValue(value: unknown, path: string, max = 512): string {
  if (typeof value !== 'string' || !wellFormedUnicode(value)) {
    fail('E_SEMANTIC_FIELD', path, 'expected a well-formed Unicode string')
  }
  if (value.length > max) fail('E_SEMANTIC_LIMIT', path, `string exceeds ${max} UTF-16 code units`)
  return value
}

export function finiteNumber(value: unknown, path: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) fail('E_SEMANTIC_NUMBER', path, 'expected a finite binary64 number')
  if (Object.is(value, -0)) fail('E_SEMANTIC_NUMBER', path, '-0 is not canonical; use +0')
  return value
}

export function integer(value: unknown, path: string, minimum = 0, maximum = Number.MAX_SAFE_INTEGER): number {
  const parsed = finiteNumber(value, path)
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    fail('E_SEMANTIC_NUMBER', path, `expected an integer in [${minimum}, ${maximum}]`)
  }
  return parsed
}

export function positive(value: unknown, path: string, allowZero = false): number {
  const parsed = finiteNumber(value, path)
  if (allowZero ? parsed < 0 : parsed <= 0) fail('E_SEMANTIC_NUMBER', path, allowZero ? 'expected a non-negative number' : 'expected a positive number')
  return parsed
}

export function booleanValue(value: unknown, path: string): boolean {
  if (typeof value !== 'boolean') fail('E_SEMANTIC_FIELD', path, 'expected a boolean')
  return value
}

export function arrayValue(value: unknown, path: string, maximum: number): unknown[] {
  if (!Array.isArray(value)) fail('E_SEMANTIC_SCHEMA', path, 'expected an array')
  if (value.length > maximum) fail('E_SEMANTIC_LIMIT', path, `array exceeds ${maximum} items`)
  return value
}

export function tupleNumbers(value: unknown, length: number, path: string): number[] {
  const items = arrayValue(value, path, length)
  if (items.length !== length) fail('E_SEMANTIC_FIELD', path, `expected exactly ${length} numbers`)
  return items.map((item, index) => finiteNumber(item, `${path}[${index}]`))
}

export function nullablePositive(value: unknown, path: string): number | null {
  return value === null ? null : positive(value, path)
}

export function assertSchemaVersion(value: unknown, path: string): void {
  exactKeys(value, ['major', 'minor'], path)
  if (value.major === 1 && (value.minor === 0 || value.minor === 1)) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', path, 'semantic programs before 1.2 require exact-source re-lowering')
  }
  if (value.major !== 1 || value.minor !== 2) fail('E_SEMANTIC_VERSION', path, 'only schema version 1.2 is supported')
}

export function utf8Compare(left: string, right: string): number {
  const leftBytes = new TextEncoder().encode(left)
  const rightBytes = new TextEncoder().encode(right)
  const shared = Math.min(leftBytes.length, rightBytes.length)
  for (let index = 0; index < shared; index++) {
    if (leftBytes[index] !== rightBytes[index]) return leftBytes[index] - rightBytes[index]
  }
  return leftBytes.length - rightBytes.length
}

export function validateSortedStrings(value: unknown, path: string, maximum: number, idPattern: RegExp): readonly string[] {
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

export function validateIdentityValue(value: unknown, path: string, depth = 0): asserts value is SemanticIdentityValue {
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

export function validateStructuralPath(value: unknown, path: string): readonly SemanticStructuralPathSegment[] {
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

export function samePath(left: readonly SemanticStructuralPathSegment[], right: readonly SemanticStructuralPathSegment[]): boolean {
  return left.length === right.length && left.every((segment, index) => (
    segment.kind === right[index].kind
    && segment.name === right[index].name
    && segment.ordinal === right[index].ordinal
  ))
}

export function structuralPathKey(path: readonly SemanticStructuralPathSegment[]): string {
  return path.map(segment => `${segment.kind.length}:${segment.kind}${segment.name.length}:${segment.name}${segment.ordinal}`).join('|')
}

export function validateOperations(value: unknown): readonly SemanticStaticOperation[] {
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

export function validateValueType(value: unknown, path: string): SemanticValueType {
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

export function representationEvidenceEqual(left: SemanticValueType, right: SemanticValueType): boolean {
  if (left.representation !== right.representation || left.evidence.tag !== right.evidence.tag) return false
  return left.evidence.tag === 'representation-preserving'
    || (right.evidence.tag === 'certified-approximation'
      && left.evidence.certificateProfile === right.evidence.certificateProfile
      && left.evidence.certificatePolicyHash === right.evidence.certificatePolicyHash)
}

export function sameValueType(left: SemanticValueType, right: SemanticValueType): boolean {
  return left.geometryKind === right.geometryKind && left.space === right.space
    && representationEvidenceEqual(left, right)
}

export function expectedValueType(
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

export function assertValueType(actual: SemanticValueType, expected: SemanticValueType, path: string): void {
  if (!sameValueType(actual, expected)) fail('E_SEMANTIC_TYPE', path, 'node valueType does not match its semantic transition')
}

export function nodeReference(value: unknown, path: string, before: number): number {
  const reference = integer(value, path, 0, SEMANTIC_PROGRAM_LIMITS.nodes - 1)
  if (reference >= before) fail('E_SEMANTIC_REFERENCE', path, 'DAG references must point to an earlier node')
  return reference
}

export function validateAnalyticLanguage(kind: string, contract: string, path: string): void {
  const analytic = kind.endsWith('-analytic')
  const polygonal = kind.endsWith('-polygonal')
  if (contract === 'legacy/current' && analytic) fail('E_SEMANTIC_TYPE', path, 'legacy/current requires materialized polygonal geometry')
  if (contract === 'openscad-viewer/brep-1' && polygonal) fail('E_SEMANTIC_TYPE', path, 'brep-1 topology requires analytic geometry')
}

export function validateNodes(value: unknown, contract: string): readonly SemanticNode[] {
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

export function identityValueKey(value: SemanticIdentityValue): string {
  switch (value.tag) {
    case 'undefined': return 'u'
    case 'null': return 'n'
    case 'boolean': return value.value ? 'b1' : 'b0'
    case 'number': return `d${String(value.value)}`
    case 'string': return `s${value.value.length}:${value.value}`
    case 'vector': return `v${value.items.length}[${value.items.map(identityValueKey).join(',')}]`
  }
}

export function sameDynamicSlots(left: readonly SemanticDynamicSlot[], right: readonly SemanticDynamicSlot[]): boolean {
  return left.length === right.length && left.every((slot, index) => (
    slot.name === right[index].name
    && slot.duplicateOrdinal === right[index].duplicateOrdinal
    && identityValueKey(slot.value) === identityValueKey(right[index].value)
  ))
}

export function sameDynamicBindings(left: readonly SemanticDynamicSlot[], right: readonly SemanticDynamicSlot[]): boolean {
  return left.length === right.length && left.every((slot, index) => (
    slot.name === right[index].name
    && identityValueKey(slot.value) === identityValueKey(right[index].value)
  ))
}

export function validateDiagnosticTemplates(
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
