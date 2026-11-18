import { createBrowserBrepDiagnosticExecutor } from '../../src/kernel/brepDiagnosticBrowser'
import { isBrepDiagnosticMessage, type BrepDiagnosticRequest } from '../../src/kernel/brepDiagnosticProtocol'
import { createNativeGeometryArtifact, isNativeGeometryArtifact } from '../../src/core/nativeGeometry'

const lane = createBrowserBrepDiagnosticExecutor()
const source = '// @language openscad-viewer/brep-1\nfull_round_exact([5,4,3]) cube([4,8,6]);'
function summary(receipt: Awaited<ReturnType<typeof lane.evaluate>>) {
  return {
    identity: receipt.identity,
    outputs: receipt.scene.outputs,
    metrics: receipt.scene.metrics,
    meshes: receipt.scene.result.meshes.map(mesh => {
      const snapshot = mesh.nativeGeometry!
      if (!isNativeGeometryArtifact(snapshot) || snapshot.kind !== 'rational-brep') throw new Error('Missing exact worker snapshot')
      const data = JSON.parse(snapshot.geometryJson)
      if (data.encoding !== 'exact-full-round-snapshot/1') throw new Error('Unexpected exact snapshot encoding')
      return { triangles: mesh.indices.length / 3, revision: snapshot.revision, entityId: mesh.entityId,
        coefficientType: typeof data.artifact.geometry.vertices[0].point[0][0],
        faceIdsAuthoritative: mesh.faceIdsAuthoritative, topologyFaceIds: data.artifact.geometry.topologyIds.faces }
    }),
  }
}
async function run() {
  let ticks = 0
  const timer = setInterval(() => ticks++, 10)
  try {
    const raw = await lane.evaluate(source, { quality: 'preview', segments: 4 })
    const single = summary(raw)
    const request:BrepDiagnosticRequest = {type:'evaluate',workerEpoch:raw.workerEpoch,jobId:raw.jobId,
      source,sourceSha256:raw.sourceSha256,policy:{quality:'preview',segments:4},identity:raw.identity}
    const corrupted = (kind:'brep'|'rational-brep',encoding:string) => {
      const copy=structuredClone(raw), mesh=copy.scene.result.meshes[0], original=mesh.nativeGeometry!
      const data=JSON.parse(original.geometryJson);data.encoding=encoding
      mesh.nativeGeometry=createNativeGeometryArtifact(original.nodeId,kind,data,JSON.parse(original.documentJson))
      ;(copy.scene.outputs[0] as any).snapshotRevision=mesh.nativeGeometry.revision
      return !isBrepDiagnosticMessage(copy,request)
    }
    const changedFaces=structuredClone(raw)
    ;(changedFaces.scene.outputs[0].topologyFaceIds as string[])[0]='f:forged'
    const messageChecks={wrongCarrierRejected:corrupted('brep','exact-full-round-snapshot/1'),
      wrongEncodingRejected:corrupted('rational-brep','unknown-snapshot/1'),
      faceBindingRejected:!isBrepDiagnosticMessage(changedFaces,request)}
    if(Object.values(messageChecks).some(ok=>!ok))throw new Error('Worker receipt accepted a forged exact binding')
    const multiSource = '// @language openscad-viewer/brep-1\nfull_round_exact([5,4,3]){cube([4,8,6]);cube([4,8,7]);}'
    const multi = summary(await lane.evaluate(multiSource, { quality: 'preview', segments: 4 }))
    let invalidSelection: string | null = null
    try { await lane.evaluate('// @language openscad-viewer/brep-1\nfull_round_exact([0,1,2]) cube([4,8,6]);') }
    catch (error) { invalidSelection = (error as {code?:string}).code ?? null }
    if (invalidSelection !== 'E_SEMANTIC_BACKEND_FAILURE') throw new Error('Invalid native selection did not refuse')
    const controller = new AbortController()
    const pending = lane.evaluate(source, { quality: 'preview', segments: 4 }, { signal: controller.signal })
    const cancel = setTimeout(() => controller.abort(), 50)
    let cancelled: string | null = null
    try { await pending } catch (error) { cancelled = (error as {code?:string}).code ?? null }
    finally { clearTimeout(cancel) }
    if (cancelled !== 'E_BREP_DIAGNOSTIC_CANCELLED') throw new Error('Worker cancellation did not refuse publication')
    const recovered = summary(await lane.evaluate(source, { quality: 'preview', segments: 2 }))
    return { single, multi, messageChecks, invalidSelection, cancelled, recovered, ticks, ownership: lane.snapshot() }
  } finally { clearInterval(timer) }
}
;(globalThis as any).runFullRoundWorkerQualification = run
