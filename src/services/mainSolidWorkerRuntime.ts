import {solveBondedSolid} from './bondedSolid'
import {inspectStructuralSections} from './structuralSections'
import {inspectCadPairs} from './cadInspection'
import {cadOperation} from './cadWorkbench'
import {mainOperation} from './mainModeling'
import {solveTruss} from './trussAnalysis'
import {spatialGraph} from './solidLightening'
import {flattenGroupGeometry} from './meshFlatten'
import {checkLatticeGraphInput, checkLatticeGraphMeshInput, isNominalLatticeGraph} from './latticeGraphProtocol'
import {warmGeometryKernel} from './geometry/kernel'
import {prepareMainSolidTransfer} from './mainSolidWorkerTransport'
import {createWorkerHandler} from './workerHandlerRuntime'
import type {MainSolidJob, MainSolidRequest, MainSolidResponse, MainSolidResults} from './mainSolidProtocol'

function execute(job:MainSolidJob):MainSolidResults[keyof MainSolidResults] {
  switch(job.kind) {
    case 'bondedSolid':return solveBondedSolid(job.inputJson)
    case 'structuralSections': {
      checkLatticeGraphMeshInput(job.mesh)
      return inspectStructuralSections(flattenGroupGeometry([job.mesh]),job.axis,job.stations)
    }
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

export function createMainSolidWorkerHandler(post:(response:MainSolidResponse,transfer?:ArrayBuffer[])=>void) {
  return createWorkerHandler<MainSolidRequest,MainSolidResponse,MainSolidResults[keyof MainSolidResults]>(post,{
    validate:(value)=>{
      const request=value as Partial<MainSolidRequest>|null
      if(!request || request.version!==1 || !Number.isSafeInteger(request.id) || request.id!<1
        || !request.job || !['main','cad','inspect','truss','latticeGraph','structuralSections','bondedSolid'].includes(request.job.kind))return null
      return request as MainSolidRequest
    },
    busyError:{name:'Error',code:'CAD_BUSY',message:'CAD worker is busy'},
    beforeExecute:()=>warmGeometryKernel(),
    execute:(request)=>execute(request.job),
    success:(request,result)=>{
      // Transfer only the freshly created response buffers; never the request's scene-owned ones.
      const prepared=prepareMainSolidTransfer({version:1,id:request.id,kind:request.job.kind,ok:true,result})
      return {message:prepared.response,transfer:prepared.transfer}
    },
    failure:(request,error)=>({version:1,id:request.id,kind:request.job.kind,ok:false,error}),
  })
}
