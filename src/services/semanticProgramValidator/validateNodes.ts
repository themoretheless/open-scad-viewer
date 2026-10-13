import { SEMANTIC_PROGRAM_LIMITS } from '../../core/semanticProgram'
import type { SemanticDynamicSlot, SemanticIdentityValue, SemanticNode, SemanticOccurrence, SemanticStaticOperation, SemanticValueType } from '../../core/semanticProgram'
import type { SemanticModuleActivationAnchor, SemanticOccurrenceValidationIndex } from '../semanticProgramValidator'
import { arrayValue, assertValueType, booleanValue, exactKeys, expectedValueType, fail, finiteNumber, integer, isRecord, nodeReference, positive, representationEvidenceEqual, stringValue, tupleNumbers, validateAnalyticLanguage, validateValueType } from './fail'

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

export function consumeOccurrenceProofBudget(
  validationIndex: Pick<SemanticOccurrenceValidationIndex, 'proofBudget'>,
  steps = 1,
): void {
  validationIndex.proofBudget.steps += steps
  if (validationIndex.proofBudget.steps > SEMANTIC_PROGRAM_LIMITS.snapshotValues) {
    fail('E_SEMANTIC_LIMIT', '$.core.occurrences', 'occurrence ancestry/continuation proof exceeds its bounded work budget')
  }
}

export function moduleActivationAnchor(
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
