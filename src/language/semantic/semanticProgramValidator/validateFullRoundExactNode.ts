import type { SemanticValueType } from '../../../core/semanticProgram'
import { assertValueType, arrayValue, exactKeys, fail, integer, isRecord, nodeReference } from './fail'

export function validateFullRoundExactNode(
  node: Record<string, unknown>, nodes: readonly unknown[], index: number,
  contract: string, path: string, valueType: SemanticValueType,
): void {
  if (contract !== 'openscad-viewer/brep-1') fail('E_SEMANTIC_TYPE', path, 'exact Full Round requires brep-1')
  const optional = ['limits', 'maxBytes'].filter(key => Object.hasOwn(node, key))
  exactKeys(node, ['id', 'kind', 'valueType', 'input', 'faces', ...optional], path)
  const input = nodeReference(node.input, `${path}.input`, index)
  const source = (nodes[input] as { valueType: SemanticValueType }).valueType
  assertValueType(source, { geometryKind: 'solid', space: 'd3', representation: 'analytic-brep', evidence: { tag: 'representation-preserving' } }, `${path}.input`)
  assertValueType(valueType, { geometryKind: 'solid', space: 'd3', representation: 'rational-brep', evidence: { tag: 'representation-preserving' } }, `${path}.valueType`)
  const faces = arrayValue(node.faces, `${path}.faces`, 3)
  if (faces.length !== 3) fail('E_SEMANTIC_FIELD', `${path}.faces`, 'three original face indices required')
  faces.forEach((face, i) => integer(face, `${path}.faces[${i}]`, 0, 0xffffffff))
  if (new Set(faces).size !== 3) fail('E_SEMANTIC_FIELD', `${path}.faces`, 'selected faces must be distinct')
  if (Object.hasOwn(node, 'maxBytes')) integer(node.maxBytes, `${path}.maxBytes`, 1, 4 << 20)
  if (Object.hasOwn(node, 'limits')) {
    const limits = node.limits
    if (!isRecord(limits)) fail('E_SEMANTIC_SCHEMA', `${path}.limits`, 'expected material limits object')
    const allowed = ['maxPairProjections', 'maxAtoms', 'maxRayCells', 'refinements', 'radiusHalvings']
    for (const [key, value] of Object.entries(limits)) {
      if (!allowed.includes(key)) fail('E_SEMANTIC_FIELD', `${path}.limits.${key}`, 'unknown material limit')
      integer(value, `${path}.limits.${key}`, 0, 0xffffffff)
    }
  }
}
