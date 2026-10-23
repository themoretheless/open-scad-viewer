import { loadCadKernelOps } from '../cadKernelOps'
import type { CadKernelHandle, CadKernelOps, CadKernelSolidAnalysis } from '../cadKernelOps'
import { MANIFOLD_PLAN_CARRIERS, MANIFOLD_PLAN_PAYLOAD, ManifoldPlanBackend, ManifoldPlanBackendError } from '../manifoldPlanBackend'
import type { ManifoldPlanBackendErrorCode, ManifoldPlanCarrier, ManifoldPlanPayload, ManifoldPlanPayloadFamily, ManifoldPlanPayloadView, OwnedManifoldPlanPayload } from '../manifoldPlanBackend'
import type { SemanticCarrierKey, SemanticRuntimeValue } from '../semanticProgramExecutor'
import type { SemanticNode, SemanticValueType } from '../../core/semanticProgram'

/** @internal Mesh analysis/shadow qualification boundary, never a compiler API. */
export function inspectManifoldPlanPayload(payload: ManifoldPlanPayload): ManifoldPlanPayloadView {
  const owned = payload as Partial<OwnedManifoldPlanPayload>
  if (owned[MANIFOLD_PLAN_PAYLOAD] !== true
    || owned.handle === undefined
    || owned.kernel === undefined
    || owned.provenance === undefined) {
    throw new TypeError('Value is not a Manifold plan payload')
  }
  return Object.freeze({
    carrierKey: owned.carrierKey as ManifoldPlanCarrier,
    dimension: owned.carrierKey === 'region/d2/mesh' ? 2 : 3,
    nodeByOriginalId: new Map(owned.provenance.nodeByOriginalId),
  })
}

/** @internal The only allocating mesh-analysis operation exposed to assemblers. */
export function analyzeManifoldPlanSolid(
  payload: ManifoldPlanPayload,
): CadKernelSolidAnalysis {
  const owned = payload as Partial<OwnedManifoldPlanPayload>
  if (owned[MANIFOLD_PLAN_PAYLOAD] !== true
    || owned.handle === undefined
    || owned.kernel === undefined
    || owned.carrierKey !== 'solid-set/d3/mesh') {
    throw new TypeError('Value is not a solid Manifold plan payload')
  }
  return owned.kernel.analyzeSolid(owned.handle)
}

export function carrierKey(valueType: SemanticValueType): SemanticCarrierKey {
  return `${valueType.geometryKind}/${valueType.space}/${valueType.representation}`
}

export function isSupportedCarrier(value: SemanticCarrierKey): value is ManifoldPlanCarrier {
  return (MANIFOLD_PLAN_CARRIERS as readonly string[]).includes(value)
}

export function backendError(
  code: ManifoldPlanBackendErrorCode,
  message: string,
): ManifoldPlanBackendError {
  return new ManifoldPlanBackendError(code, message)
}

export function safeDelete(
  kernel: CadKernelOps,
  handle: CadKernelHandle,
): unknown | undefined {
  try {
    kernel.delete(handle)
    return undefined
  } catch (error) {
    return error
  }
}

export function requirePayload(
  input: SemanticRuntimeValue<ManifoldPlanPayloadFamily> | undefined,
  expectedDimension: 2 | 3,
  node: SemanticNode,
): OwnedManifoldPlanPayload {
  if (input === undefined || input.tag === 'empty') {
    throw backendError('E_MANIFOLD_PLAN_CONTRACT', `${node.kind} received a missing input after shared empty reduction`)
  }
  const payload = input.payload as Partial<OwnedManifoldPlanPayload>
  if (payload[MANIFOLD_PLAN_PAYLOAD] !== true
    || payload.handle === undefined
    || payload.kernel === undefined) {
    throw backendError('E_MANIFOLD_PLAN_CONTRACT', `${node.kind} received a foreign Manifold payload`)
  }
  const dimension = payload.carrierKey === 'region/d2/mesh' ? 2 : 3
  if (dimension !== expectedDimension) {
    throw backendError('E_MANIFOLD_PLAN_CONTRACT', `${node.kind} received a ${dimension}D input where ${expectedDimension}D was required`)
  }
  return payload as OwnedManifoldPlanPayload
}

export function matrix3FromMatrix4(matrix: readonly number[]): readonly number[] {
  return [
    matrix[0], matrix[1], 0,
    matrix[4], matrix[5], 0,
    matrix[12], matrix[13], 1,
  ]
}

/** Load a non-instrumented module so ownership stays per lease, not global. */
export async function loadManifoldPlanBackend(): Promise<ManifoldPlanBackend> {
  return new ManifoldPlanBackend(await loadCadKernelOps())
}
