import type { CallNode } from '../../openscadCompiler'
import type { EvalContext, SemanticShape } from '../openscadSemanticLowerer'
import { evaluationError } from '../../openScadStableExpressionEval'
import { arg } from './evalExpression'
import { frameChildren, invoke, nodeValueType } from './evalPreparedNodes'

export function evalProjection(node: CallNode, ctx: EvalContext): SemanticShape[] {
  const cut = arg(node, 'cut', 0, false, ctx) === true
  const occurrence = invoke(node, ctx)
  return frameChildren(node, node.children, occurrence, ctx).map(shape => {
    if (shape.dimension !== 'solid3') evaluationError(ctx, node.p, 'projection() requires 3D children')
    return ctx.builder.produce(occurrence, {
      kind: 'projection', valueType: { ...nodeValueType(shape, ctx), geometryKind: 'region', space: 'd2' },
      input: shape.node, cut,
    }, shape.color)
  })
}
