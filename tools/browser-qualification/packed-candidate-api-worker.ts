/// <reference lib="webworker" />
// Independent realm: deliberately ignore any module supplied by the host.
// No optional network compiler is installed: decode and verify embedded bytes.
const workerScope=self as DedicatedWorkerGlobalScope
workerScope.onmessage=async event=>{
 try{
  const kernel=await import('../../src/kernel/geometry/kernel')
  await kernel.warmGeometryKernel()
  const {default:identity}=await import('../../src/generated/geometry-kernels/identity')
  if(identity.sha256!==event.data.identity.sha256||identity.byteLength!==event.data.identity.byteLength)throw Error('Independent packed artifact identity mismatch')
  const {runCandidateFixtures}=await import('./candidate-direct-api-fixtures')
  workerScope.postMessage({ok:true,identity,result:runCandidateFixtures()})
 }catch(error){workerScope.postMessage({ok:false,error:String(error)})}
}
export {}
