import { evaluateExactSolids } from '../geometryBuildEngine'
import { stringifyMeshJson } from '../meshJson'
import { buildExactSolidBodies } from './brepBuild'
import { EXACT_SOLID_MAX_DOCUMENT_CHARACTERS, isExactSolidRequest, type ExactSolidResponse } from './exactSolidProtocol'

/** Runs entirely in the geometry worker; only detached JSON crosses the boundary. */
export async function runExactSolidRequest(request: unknown): Promise<ExactSolidResponse> {
  try {
    if (!isExactSolidRequest(request)) throw new Error('Invalid exact-solid request.')
    const evaluated = await evaluateExactSolids(request.source)
    const plan = evaluated.exactSolids
    if (plan && plan.roots.length > 200) throw new Error('An exact-solid group is limited to 200 bodies.')
    const bodies = plan ? buildExactSolidBodies(plan.nodes, plan.roots) : []
    const document = stringifyMeshJson({ version: 1, sketches: [], bodies })
    if (document.length > EXACT_SOLID_MAX_DOCUMENT_CHARACTERS) throw new Error('Document exceeds 64 MB.')
    return { kind: 'exact-solid', version: 1, ok: true, document }
  } catch (error) {
    return {
      kind: 'exact-solid', version: 1, ok: false,
      error: {
        name: (error instanceof Error ? error.name : 'Error').slice(0, 100),
        message: (error instanceof Error ? error.message : String(error)).slice(0, 4096),
      },
    }
  }
}
