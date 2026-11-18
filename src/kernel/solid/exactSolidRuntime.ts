import { sceneMeshesToSolidDocument } from '../solidBridge'
import { evaluateExactSolids } from '../geometryBuildEngine'
import { stringifyMeshJson } from '../../exchange/meshJson'
import { buildExactSolidBodies } from './brepBuild'
import { codedError, errorCode } from '../../i18n/kernelErrors'
import { EXACT_SOLID_MAX_DOCUMENT_CHARACTERS, isExactSolidRequest, type ExactSolidResponse } from './exactSolidProtocol'

/** Runs entirely in the geometry worker; only detached JSON crosses the boundary. */
export async function runExactSolidRequest(request: unknown): Promise<ExactSolidResponse> {
  try {
    if (!isExactSolidRequest(request)) throw codedError('CAD_PROTOCOL', 'Invalid exact-solid request.')
    const evaluated = await evaluateExactSolids(request.source)
    const plan = evaluated.exactSolids
    if (plan && plan.roots.length > 200)
      throw codedError('EXACT_SOLID_BODY_LIMIT', 'An exact-solid group is limited to 200 bodies.')
    let bodies
    if (plan) bodies = buildExactSolidBodies(plan.nodes, plan.roots)
    else {
      if (evaluated.meshes.length > 200)
        throw codedError('EXACT_SOLID_BODY_LIMIT', 'An exact-solid group is limited to 200 bodies.')
      if (evaluated.meshes.some(mesh => mesh.nativeGeometry?.kind !== 'brep'))
        throw codedError('EXACT_SOLID_NOT_BREP', 'Solid source requires native B-rep bodies.')
      bodies = sceneMeshesToSolidDocument(evaluated.meshes).bodies
    }
    if (bodies.length > 200)
      throw codedError('EXACT_SOLID_BODY_LIMIT', 'An exact-solid group is limited to 200 bodies.')
    const document = stringifyMeshJson({ version: 1, sketches: [], bodies })
    if (document.length > EXACT_SOLID_MAX_DOCUMENT_CHARACTERS)
      throw codedError('EXACT_SOLID_DOCUMENT_LIMIT', 'Document exceeds 64 MB.')
    return { kind: 'exact-solid', version: 1, ok: true, document }
  } catch (error) {
    const code = errorCode(error)
    return {
      kind: 'exact-solid',
      version: 1,
      ok: false,
      error: {
        name: (error instanceof Error ? error.name : 'Error').slice(0, 100),
        message: (error instanceof Error ? error.message : String(error)).slice(0, 4096),
        ...(code && /^[A-Z][A-Z0-9_]{0,79}$/.test(code) ? { code } : {}),
      },
    }
  }
}
