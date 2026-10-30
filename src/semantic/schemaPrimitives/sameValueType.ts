// schemaPrimitives: sameValueType.
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
} from '../../core/semanticProgram'
import { representationEvidenceEqual, fail, integer, arrayValue, isRecord, stringValue, validateValueType, exactKeys, tupleNumbers, booleanValue, positive, finiteNumber, validateIdentityValue } from './SemanticProgramValidationError'


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
