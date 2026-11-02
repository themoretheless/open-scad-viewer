import type { SemanticNode } from '../core/semanticProgram'

export function remapSemanticNode(node: SemanticNode, id: number, oldToNew: Int32Array): SemanticNode {
  const remap = (input: number) => {
    const mapped = oldToNew[input]
    if (mapped < 0) throw new Error('Semantic DAG canonicalization encountered an unreachable input')
    return mapped
  }
  switch (node.kind) {
    case 'transform':
    case 'linear-extrude':
    case 'rotate-extrude-analytic':
    case 'rotate-extrude-polygonal':
    case 'projection':
    case 'offset':
    case 'full-round-exact':
      return { ...node, id, input: remap(node.input) }
    case 'boolean':
    case 'hull':
      return { ...node, id, inputs: node.inputs.map(remap) }
    default:
      return { ...node, id }
  }
}
