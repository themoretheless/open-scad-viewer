import { z } from 'zod/v4'
import type { NurbsBrep } from '../kernel/geometry/brep'
import { warmGeometryKernel } from '../kernel/geometry/kernel'
import { warmLanguageKernel } from '../language/languages/kernel'
import { compileRushFrontend } from '../language/rushFrontend'
import { compileRushGraphNurbs } from '../language/rushGraphNurbs'
import { ensureRawSourceLoader } from './ensureRawSourceLoader'
import { InvalidToolInputError } from './publicError'

export const brepSourceInputSchema = z.union([
  z
    .object({
      nurbs_document: z.record(z.string(), z.unknown()),
      nurbs_source: z.never().optional(),
      step_text: z.never().optional(),
      iges_text: z.never().optional(),
    })
    .strict(),
  z
    .object({
      nurbs_document: z.never().optional(),
      nurbs_source: z.string().min(1).max(262_144),
      step_text: z.never().optional(),
      iges_text: z.never().optional(),
    })
    .strict(),
  z
    .object({
      nurbs_document: z.never().optional(),
      nurbs_source: z.never().optional(),
      step_text: z
        .string()
        .min(10)
        .max(8 * 1024 * 1024),
      iges_text: z.never().optional(),
    })
    .strict(),
  z
    .object({
      nurbs_document: z.never().optional(),
      nurbs_source: z.never().optional(),
      step_text: z.never().optional(),
      iges_text: z
        .string()
        .min(10)
        .max(8 * 1024 * 1024),
    })
    .strict(),
])

export type BrepSourceInput = z.infer<typeof brepSourceInputSchema>

async function resolveNurbsDocumentToBrep(document: unknown): Promise<NurbsBrep> {
  let compiled: ReturnType<typeof compileRushGraphNurbs>
  try {
    compiled = compileRushGraphNurbs(document)
  } catch (error) {
    throw new InvalidToolInputError(error instanceof Error ? error.message : 'Invalid rush/nurbs-1 document.')
  }
  ensureRawSourceLoader()
  const { createOwnNurbsResolver } = await import('../language/rushGraphNurbsKernel/createOwnNurbsResolver')
  const nodes = new Map(compiled.resolved_document.nodes.map(node => [node.id, node]))
  const resolver = createOwnNurbsResolver(nodes, { action: 'build' })
  let root: ReturnType<typeof resolver.get>
  try {
    root = resolver.get(compiled.document.root)
  } catch (error) {
    throw new InvalidToolInputError(error instanceof Error ? error.message : 'Failed to build NURBS graph.', {
      code: 'invalid_geometry',
    })
  }
  if (root.kind !== 'brep') {
    throw new InvalidToolInputError(
      `Expected an exact B-rep root node, got "${root.kind}". Use brep_box, brep_sphere, brep_cylinder, brep_extrude_curves, etc.`,
    )
  }
  return root.data
}

export async function resolveBrepFromInput(input: BrepSourceInput): Promise<NurbsBrep> {
  ensureRawSourceLoader()
  await Promise.all([warmLanguageKernel(), warmGeometryKernel()])
  if ('nurbs_document' in input && input.nurbs_document !== undefined) {
    return resolveNurbsDocumentToBrep(input.nurbs_document)
  }
  if ('nurbs_source' in input && input.nurbs_source !== undefined) {
    let compiled: ReturnType<typeof compileRushFrontend>
    try {
      compiled = compileRushFrontend(input.nurbs_source)
    } catch (error) {
      throw new InvalidToolInputError(error instanceof Error ? error.message : 'Failed to compile Rush source.', {
        code: 'source_syntax_error',
      })
    }
    if (compiled.execution_target !== 'own-nurbs') {
      throw new InvalidToolInputError(
        'Rush source did not produce an own-nurbs B-rep document (ensure `// @rush/1` and B-rep operations are used).',
      )
    }
    return resolveNurbsDocumentToBrep(compiled.document)
  }
  if ('step_text' in input && input.step_text !== undefined) {
    const { importDirectStepV9 } = await import('../kernel/cadNurbsStep')
    try {
      return importDirectStepV9(input.step_text).model
    } catch (error) {
      throw new InvalidToolInputError(error instanceof Error ? error.message : 'STEP import failed.', {
        code: 'invalid_geometry',
      })
    }
  }
  const { importDirectIgesV2 } = await import('../kernel/cadIges')
  try {
    return importDirectIgesV2(input.iges_text).model
  } catch (error) {
    throw new InvalidToolInputError(error instanceof Error ? error.message : 'IGES import failed.', {
      code: 'invalid_geometry',
    })
  }
}
