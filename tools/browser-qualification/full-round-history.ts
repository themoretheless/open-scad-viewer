import {ExactFullRoundHistoryBrowser} from '../../src/services/exactFullRoundHistoryBrowser'
import {buildBrepSemanticScene} from '../../src/services/brepSemanticScene'
import {lowerOpenSCADToSemanticProgram} from '../../src/services/semanticProgramLowerer'
import {restoreExactFullRoundSnapshot} from '../../src/services/exactFullRoundSnapshot'
import {callGeometryRust,warmGeometryKernel} from '../../src/services/geometry/kernel'
async function qualification(){
 await warmGeometryKernel()
 const scene=await buildBrepSemanticScene(lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\nfull_round_exact([5,4,3]) cube([4,8,6]);'),{quality:'preview',segments:4})
 const original=scene.result.meshes[0],saved=JSON.stringify(original.nativeGeometry)
 const artifact=restoreExactFullRoundSnapshot(original.nativeGeometry!).artifact
 const source=callGeometryRust<any>('brep_nurbs_transform',{model:artifact.source,matrix:[[1,0,0,0],[0,1,0,0],[0,0,1.25,0],[0,0,0,1]]})
 const lane=new ExactFullRoundHistoryBrowser()
 let ticks=0;const timer=setInterval(()=>ticks++,5)
 const error=async(p:Promise<unknown>)=>{try{await p;throw new Error('Expected refusal')}catch(e){return String(e)}}
 try{
  const edited=await lane.edit(original,{action:'rebase-source',source})
  const undone=await lane.edit(edited,{action:'undo'})
  const faceIds=[3,2,5].map(i=>(artifact.source as any).topologyIds.faces[i]) as [string,string,string]
  const regenerated=await lane.edit(edited,{action:'regenerate-selection',revision:0,faceIds})
  const controller=new AbortController()
  const pending=lane.edit(edited,{action:'redo'},0,{signal:controller.signal})
  // Invalid sampling is rejected before a Worker starts.
  const malformed=await error(pending)
  const running=lane.edit(edited,{action:'restore'},4,{signal:controller.signal})
  const busy=await error(lane.edit(original,{action:'restore'}))
  setTimeout(()=>controller.abort(),50)
  const cancelled=await error(running)
  const deadline=await error(lane.edit(original,{action:'restore'},4,{deadlineMs:1}))
  const failed=await error(lane.edit(original,{action:'undo'}))
  const recovered=await lane.edit(undone,{action:'redo'},8)
  return {ownership:lane.snapshot(),ticks,busy,cancelled,deadline,failed,malformed,
   regenerated:regenerated.nativeGeometry!.revision!==edited.nativeGeometry!.revision,
   changed:edited.geometryAssetId!==original.geometryAssetId,
   undoExact:undone.nativeGeometry!.revision===original.nativeGeometry!.revision,
   redoExact:recovered.nativeGeometry!.revision===edited.nativeGeometry!.revision,
   inputIntact:JSON.stringify(original.nativeGeometry)===saved&&original.vertices.byteLength>0,
   moreTriangles:recovered.indices.length>edited.indices.length}
 }finally{clearInterval(timer)}
}
Object.assign(globalThis,{runHistoryQualification:qualification})
