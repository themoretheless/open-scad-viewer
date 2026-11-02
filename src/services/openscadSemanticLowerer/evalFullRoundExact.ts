import type { CallNode } from '../openscadCompiler'
import type { EvalContext, SemanticShape } from '../openscadSemanticLowerer'
import { evaluationError } from '../openScadStableExpressionEval'
import { arg } from './evalExpression'
import { frameChildren, invoke, nodeValueType } from './evalPreparedNodes'

export function evalFullRoundExact(node: CallNode, ctx: EvalContext): SemanticShape[] {
  if (ctx.builder.languageContract !== 'openscad-viewer/brep-1') {
    evaluationError(ctx, node.p, 'full_round_exact() requires openscad-viewer/brep-1')
  }
  const selected = arg(node, 'faces', 0, [], ctx)
  if (!Array.isArray(selected) || selected.length !== 3
    || selected.some(face => typeof face !== 'number' || !Number.isInteger(face) || face < 0 || face > 0xffffffff)
    || new Set(selected).size !== 3) {
    evaluationError(ctx, node.p, 'full_round_exact() requires three distinct original face indices')
  }
  const faces = selected as [number, number, number]
  const occurrence = invoke(node, ctx)
  return frameChildren(node, node.children, occurrence, ctx).map(shape => {
    const source = nodeValueType(shape, ctx)
    if (source.geometryKind !== 'solid' || source.space !== 'd3'
      || source.representation !== 'analytic-brep' || source.evidence.tag !== 'representation-preserving') {
      evaluationError(ctx, node.p, 'full_round_exact() requires an original analytic solid child')
    }
    return ctx.builder.produce(occurrence, {
      kind: 'full-round-exact', input: shape.node, faces,
      valueType: { geometryKind: 'solid', space: 'd3', representation: 'rational-brep', evidence: { tag: 'representation-preserving' } },
    }, shape.color)
  })
}
