import {meshTransferables,type MeshData} from '../core/mesh'
import type {PolygonMesh} from './geometry/polygon'
import type {KernelManifoldReport} from './geometry/manifoldAnalysis'
import type {DisplayMesh} from './solidDisplayCache'
import type { MainSolidJob, MainSolidResponse, MainSolidResults } from './mainSolidProtocol'
import type { StructuralSections } from './structuralSections'
import type { DirectDocument } from './directModeling'
import type { MeshBooleanResult, MeshRepairResult } from './meshEditing'
import type { ImportedMesh } from './meshImport'

/**
 * Wire packing for restored documents and the structural-sections source mesh
 * (up to 900k positions / 300k indices, i.e. roughly 8 MB as numbers).
 * PolygonMesh fields are Float64Array/Uint32Array end to end now, so the worker
 * transfers the freshly created response buffers directly and the host receives a
 * zero-copy move instead of an elementwise structuredClone. Float64Array keeps
 * positions bit-exact; indices are bounded by positions.length / 3, far below 2^32.
 *
 * Only these freshly created response buffers are transferred. Request buffers stay
 * owned by the scene/renderer (transferring them would detach live scene geometry),
 * mirroring the gcodePreviewWorkerTransport precedent and the svgWorkerClient note.
 */
function pushTransferView(transfer: ArrayBuffer[], view: unknown): void {
  if (view && ArrayBuffer.isView(view)) {
    const buf = view.buffer
    if (buf instanceof ArrayBuffer && !transfer.includes(buf)) {
      transfer.push(buf)
    }
  }
}

function pushManifoldReportTransfer(transfer: ArrayBuffer[], report: KernelManifoldReport): void {
  for (const view of [report.boundaryEdges, report.nonManifoldEdges, report.orientationEdges, report.degenerateTriangles, report.nonManifoldVertices, report.isolatedVertices]) {
    pushTransferView(transfer, view)
  }
}

export function prepareMainSolidTransfer(response: MainSolidResponse): { response: MainSolidResponse; transfer: ArrayBuffer[] } {
  if(response.ok&&response.kind==='trussField')return {response,transfer:meshTransferables(response.result as MeshData[])}
  if(response.ok && (response.kind==='displayMesh'||response.kind==='surfaceMesh')) {
    const mesh=response.kind==='surfaceMesh'?response.result as PolygonMesh:(response.result as DisplayMesh).mesh
    const transfer: ArrayBuffer[] = []
    pushTransferView(transfer, mesh.positions)
    pushTransferView(transfer, mesh.indices)
    pushTransferView(transfer, mesh.uv)
    return {response,transfer}
  }
  if(response.ok&&response.kind==='meshManifold'){
    const entries=response.result as MainSolidResults['meshManifold'],transfer:ArrayBuffer[]=[]
    for(const entry of entries)pushManifoldReportTransfer(transfer,entry.report)
    return {response,transfer}
  }
  if(response.ok&&response.kind==='meshRepair'){
    const result=response.result as MeshRepairResult,transfer:ArrayBuffer[]=[]
    for(const mesh of [result.mesh,result.repair]){pushTransferView(transfer,mesh?.positions);pushTransferView(transfer,mesh?.indices)}
    for(const report of [result.before,result.after])pushManifoldReportTransfer(transfer,report)
    return {response,transfer}
  }
  if(response.ok&&response.kind==='meshBoolean'){
    const result=response.result as MeshBooleanResult,transfer:ArrayBuffer[]=[]
    pushTransferView(transfer,result.mesh.positions);pushTransferView(transfer,result.mesh.indices)
    return {response,transfer}
  }
  if(response.ok&&response.kind==='meshImport'){
    const result=response.result as ImportedMesh,transfer:ArrayBuffer[]=[]
    pushTransferView(transfer,result.positions);pushTransferView(transfer,result.indices)
    for(const report of [result.manifold.before,result.manifold.after])pushManifoldReportTransfer(transfer,report)
    return {response,transfer}
  }
  if(response.ok&&(response.kind==='meshConvert'||response.kind==='meshExport')){
    const result=response.result as MainSolidResults['meshConvert'|'meshExport'],transfer:ArrayBuffer[]=[]
    pushTransferView(transfer,result.data)
    return {response,transfer}
  }
  if(response.ok && (response.kind==='restoreDocument'||response.kind==='rushGraphImport'||response.kind==='primitive'||response.kind==='sceneEdit')) {
    // These jobs validate a complete replacement, so parsed mesh buffers belong
    // exclusively to the response. Deduplicate buffers in case views share backing storage.
    const bodies = (response.result as DirectDocument).bodies
    if (bodies.length <= 8) {
      const transfer: ArrayBuffer[] = []
      for (let i = 0; i < bodies.length; i++) {
        const mesh = bodies[i]!.mesh
        pushTransferView(transfer, mesh.positions)
        pushTransferView(transfer, mesh.indices)
        pushTransferView(transfer, mesh.uv)
      }
      return { response, transfer }
    }
    const buffers=new Set<ArrayBuffer>()
    for(const body of bodies) {
      const mesh = body.mesh
      if(mesh.positions && ArrayBuffer.isView(mesh.positions) && mesh.positions.buffer instanceof ArrayBuffer)buffers.add(mesh.positions.buffer)
      if(mesh.indices && ArrayBuffer.isView(mesh.indices) && mesh.indices.buffer instanceof ArrayBuffer)buffers.add(mesh.indices.buffer)
      if(mesh.uv && ArrayBuffer.isView(mesh.uv) && mesh.uv.buffer instanceof ArrayBuffer)buffers.add(mesh.uv.buffer)
    }
    return {response,transfer:[...buffers]}
  }
  if (!response.ok || response.kind !== 'structuralSections') return { response, transfer: [] }
  const result = response.result as StructuralSections
  const mesh = result?.sourceMesh
  if (!mesh || !(mesh.positions instanceof Float64Array) || !(mesh.indices instanceof Uint32Array)) return { response, transfer: [] }
  const posBuf = mesh.positions.buffer as ArrayBuffer
  const idxBuf = mesh.indices.buffer as ArrayBuffer
  return { response, transfer: posBuf === idxBuf ? [posBuf] : [posBuf, idxBuf] }
}

/** Typed arrays survive structuredClone intact, so protocol validation sees them
 * directly; this stays as the seam for any future packing. */
export function decodeMainSolidResult(kind: MainSolidJob['kind'], result: unknown): unknown {
  return result
}
