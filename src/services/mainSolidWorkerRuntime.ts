import {inspectCadPairs} from './cadInspection'
import {cadOperation} from './cadWorkbench'
import {mainOperation} from './mainModeling'
import {solveTruss} from './trussAnalysis'
import {spatialGraph} from './solidLightening'
import {flattenGroupGeometry} from './meshFlatten'
import {checkLatticeGraphInput, isNominalLatticeGraph} from './latticeGraphProtocol'
import {warmGeometryKernel} from './geometry/kernel'
import type {MainSolidJob, MainSolidRequest, MainSolidResponse, MainSolidResults} from './mainSolidProtocol'

function execute(job:MainSolidJob):MainSolidResults[keyof MainSolidResults] {
  switch(job.kind) {
    case 'inspect':return inspectCadPairs(job.bodies)
    case 'cad':return cadOperation(job.document,job.options)
    case 'main':return mainOperation(job.meshes,job.selected,job.hit,job.operation,job.parameters)
    case 'truss':return solveTruss(job.model)
    case 'latticeGraph': {
      checkLatticeGraphInput(job.mesh,job.options)
      const graph={modelKind:'nominal-bounding-box-axial' as const,...spatialGraph({id:'nominal',name:'Nominal graph',mesh:flattenGroupGeometry([job.mesh])},job.options)}
      if(!isNominalLatticeGraph(graph))throw new Error('Nominal graph requires distinct nodes and three-dimensional bounds within 125 nodes and 400 members.')
      return graph
    }
  }
}

export function createMainSolidWorkerHandler(post:(response:MainSolidResponse)=>void) {
  let active=false
  return async(value:unknown)=>{
    const request=value as Partial<MainSolidRequest>|null
    if(!request || request.version!==1 || !Number.isSafeInteger(request.id) || request.id!<1
      || !request.job || !['main','cad','inspect','truss','latticeGraph'].includes(request.job.kind))return
    const {job}=request, envelope={version:1 as const,id:request.id!,kind:job.kind}
    if(active){post({...envelope,ok:false,error:{name:'Error',code:'CAD_BUSY',message:'CAD worker is busy'}});return}
    active=true
    try {
      await warmGeometryKernel()
      post({...envelope,ok:true,result:execute(job)})
    } catch(cause) {
      const error=cause instanceof Error?cause:new Error(String(cause))
      post({...envelope,ok:false,error:{name:error.name,message:error.message,
        ...('code' in error && typeof error.code==='string'?{code:error.code}:{})}})
    } finally {active=false}
  }
}
