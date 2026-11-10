// The RushGraph frontends are a separate WASM kernel. Keeping this importer out of
// `solidNurbs.ts` keeps that kernel off the startup graph: it is reached only through a
// dynamic import at the single call site that opens a RushGraph document.
import { buildOwnNurbs } from '../language/rushGraphNurbsKernel'
import type { NurbsCurve } from '../kernel/nurbsCurve'
import { validateNurbsCurve } from '../kernel/nurbsCurve'
import type { NurbsSurface } from '../kernel/nurbsSurface'
import { validateNurbsSurface } from '../kernel/nurbsSurface'
import type { SolidDocumentWithNurbs, SolidNurbsCurve, SolidNurbsSurface } from '../kernel/solidNurbs'

export function importRushGraphNurbs(document: unknown): Pick<SolidDocumentWithNurbs, 'curves' | 'surfaces'> {
  const built = buildOwnNurbs(document, { action: 'build', display: { segments: 16, subdivisionLevels: 1 } })
  const definitions = built.report.definitions as Record<string, ({ kind: string } & Record<string, unknown>)>
  const curves: SolidNurbsCurve[] = []
  const surfaces: SolidNurbsSurface[] = []
  for (const [nodeId, definition] of Object.entries(definitions)) {
    const { kind, ...data } = definition
    if (kind === 'curve') {
      const curve = data as unknown as NurbsCurve
      validateNurbsCurve(curve)
      curves.push({ id: crypto.randomUUID(), name: nodeId, curve })
    } else if (kind === 'surface') {
      const surface = data as unknown as NurbsSurface
      validateNurbsSurface(surface)
      surfaces.push({ id: crypto.randomUUID(), name: nodeId, surface, segmentsU: 16, segmentsV: 16 })
    }
  }
  if (!curves.length && !surfaces.length) throw new Error('RushGraph has no reachable NURBS curves or surfaces.')
  return { curves, surfaces }
}
