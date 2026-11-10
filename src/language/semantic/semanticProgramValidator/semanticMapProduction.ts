import type { SemanticNode } from '../../../core/semanticProgram'
import { stableJson } from '../../../core/semanticProgram/semanticResultItems'
import type { SemanticProductionRule } from '../semanticProgramValidator'
import { fail } from './fail'

export function isSemanticMapRule(rule: SemanticProductionRule): boolean {
  return ['transform-map', 'projection-map', 'offset-map', 'full-round-exact-map'].includes(rule)
}
export function semanticMapNodeKinds(rule: SemanticProductionRule): readonly SemanticNode['kind'][] {
  switch (rule) {
    case 'transform-map': return ['transform']
    case 'projection-map': return ['projection']
    case 'offset-map': return ['offset']
    case 'full-round-exact-map': return ['full-round-exact']
    default: return fail('E_SEMANTIC_IDENTITY', '$.core.operations', 'unexpected map production rule')
  }
}
export function semanticMapParameters(node: SemanticNode): string {
  switch (node.kind) {
    case 'transform': return JSON.stringify(node.matrix)
    case 'projection': return String(node.cut)
    case 'offset': return String(node.distance)
    case 'full-round-exact': return stableJson({ faces: node.faces,
      ...(node.limits === undefined ? {} : { limits: node.limits }),
      ...(node.maxBytes === undefined ? {} : { maxBytes: node.maxBytes }),
    })
    default: return fail('E_SEMANTIC_IDENTITY', `$.core.nodes[${node.id}]`, 'unexpected map node kind')
  }
}
