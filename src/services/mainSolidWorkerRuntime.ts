import {measureSolidDistance} from './solidDistance'
import {inspectSelfIntersection} from './solidSelfIntersection'
import {inspectFaceContacts} from './solidFaceContacts'
import {inspectBoundaryAgreement} from './solidBoundaryAgreement'
import {measureNurbsSurfaceDistance} from './nurbsSurface'
import {measureNurbsCurveDistance} from './nurbsCurve'
import {sketchSnapGeometry} from './modelingSnaps'
import {bodySnapGeometry} from './solidSnapGeometry'
import {prepareSolidFaceSketch} from './solidFaceSketch'
import {solidBodyEdges} from './solidBodyEdges'
import {solidTopology} from './directSolidTools'
import {retainedProfileDisplay} from './retainedSketchProfile'
import {tessellateSolidNurbsSurface,sampleSolidNurbsCurve} from './solidNurbs'
import {measureSurfaceBoundaries} from './solidSurfaceDiagnostics'
import {measureSolidVertices,measureSolidEdgeCurvature,measureFaceDistance,measureShellDistance} from './solidMeasurements'
import {addSolidPrimitive} from './solidPrimitive'
import {prepareSolidDisplay} from './solidDisplayPreparation'
import {SolidInstanceBatchCache} from './solidInstanceBatchCache'
import {parseDirectDocument} from './directModeling'
import {applySolidBrepTool} from './solidBrepTool'
import {applySolidPointEdit} from './solidPointEdit'
import {applySolidSketchEdit} from './solidSketchEdit'
import {applySolidBoolean} from './solidBoolean'
import {applySolidSceneEdit} from './solidSceneEdit'
import {matchSolidCurve} from './solidCurveMatching'
import {matchSolidSurface,prepareSolidSurfaceSeams} from './solidSurfaceMatching'
import {buildSolidSurface} from './solidSurfaceConstruction'
import {refitSolidNurbs} from './solidCurveReduction'
import {prepareSolidProfile} from './solidProfilePreparation'
import {applySolidProfileEdit} from './solidProfileEdit'
import {applySolidBodyEdit} from './solidBodyEdit'
import {applySolidRevolve} from './solidRevolve'
import {applyDirectExtrusionProfile} from './directExtrusion'
import {inspectSolidIntersections} from './solidDiagnostics'
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

async function execute(job:MainSolidJob):Promise<MainSolidResults[keyof MainSolidResults]> {
  switch(job.kind) {
    case 'sketchSnaps':return sketchSnapGeometry([job.sketch])
    case 'bodySnaps':return bodySnapGeometry(job.body)
    case 'faceSketch':return prepareSolidFaceSketch(job.body,job.face)
    case 'bodyEdges':return solidBodyEdges(job.body)
    case 'topology':return solidTopology(job.mesh)
    case 'curveDisplay':return sampleSolidNurbsCurve(job.curve)
    case 'profileDisplay':return retainedProfileDisplay(job.profile)
    case 'surfaceMesh':return tessellateSolidNurbsSurface(job.item)
    case 'surfaceBoundary':return measureSurfaceBoundaries(job.a,job.b,job.options)
    case 'selfIntersection':return inspectSelfIntersection(job.model,job.toleranceUv,job.limits,job.maxSpans)
    case 'faceContacts':return inspectFaceContacts(job.model,job.toleranceUv,job.limits)
    case 'boundaryAgreement':return inspectBoundaryAgreement(job.model,job.maxCells)
    case 'solidDistance':return measureSolidDistance(job.options)
    case 'shellDistance':return measureShellDistance(job.options)
    case 'faceDistance':return measureFaceDistance(job.options)
    case 'surfaceDistance':return measureNurbsSurfaceDistance(job.a,job.b,job.toleranceMm,job.maxCells)
    case 'curveDistance':return measureNurbsCurveDistance(job.a,job.b,job.toleranceMm,job.maxCells)
    case 'measureVertices':return measureSolidVertices(job.a,job.indexA,job.b,job.indexB)
    case 'measureEdge':return measureSolidEdgeCurvature(job.body,job.edge,job.parameter)
    case 'primitive':return addSolidPrimitive(job.document,job.options)
    case 'displayMesh':return prepareSolidDisplay(job.mesh,job.brep,job.segments)
    case 'restoreDocument':return parseDirectDocument(job.text,instanceCache)
    case 'modelGraphImport':return (await import('./solidModelGraphImport')).importSolidModelGraph(job.document,job.text,job.group)
    case 'brepTool':return applySolidBrepTool(job.document,job.options)
    case 'curveChainInspection':return (await import('./inspectCurrentCurveChain')).inspectCurrentCurveChain(job.document,job.ids,job.maxPairs)
    case 'curveOffset':return (await import('./solidCurveOffset')).offsetSolidCurve(job.document,job.options)
    case 'nurbsEdit':return (await import('./solidNurbsEdit')).applySolidNurbsEdit(job.document,job.options)
    case 'pointEdit':return applySolidPointEdit(job.document,job.options)
    case 'sketchEdit':return applySolidSketchEdit(job.document,job.options)
    case 'boolean':return applySolidBoolean(job.document,job.options)
    case 'sceneEdit':return applySolidSceneEdit(job.document,job.options)
    case 'curveMatch':return matchSolidCurve(...job.args)
    case 'surfaceMatch':return matchSolidSurface(...job.args)
    case 'seamPrepare':return prepareSolidSurfaceSeams(...job.args)
    case 'surfaceBuild':return buildSolidSurface(job.document,job.options)
    case 'nurbsRefit':return refitSolidNurbs(job.document,job.options)
    case 'profilePrepare':return prepareSolidProfile(job.document,job.ids,job.tolerance)
    case 'profileEdit':return applySolidProfileEdit(job.document,job.options)
    case 'bodyEdit':return applySolidBodyEdit(job.document,job.options)
    case 'revolve':return applySolidRevolve(job.document,job.options)
    case 'extrusion':return applyDirectExtrusionProfile(job.document,job.options)
    case 'bondedSolid':return solveBondedSolid(job.inputJson)
    case 'structuralSections': {
      checkLatticeGraphMeshInput(job.mesh)
      return inspectStructuralSections(flattenGroupGeometry([job.mesh]),job.axis,job.stations)
    }
    case 'meshContacts':return inspectSolidIntersections(job.mesh,{maxWork:job.maxWork,maxContacts:job.maxContacts})
    case 'inspect':return inspectCadPairs(job.bodies)
    case 'cad':return cadOperation(job.document,job.options)
    case 'main':return mainOperation(job.meshes,job.selected,job.hit,job.operation,job.parameters)
    case 'truss':return solveTruss(job.model)
    case 'latticeGraph': {
      checkLatticeGraphInput(job.mesh,job.options)
      const graph={modelKind:'nominal-bounding-box-axial' as const,...spatialGraph({id:'nominal',name:'Nominal graph',mesh:flattenGroupGeometry([job.mesh])},job.options)}
      if(!isNominalLatticeGraph(graph))throw new Error('Graph needs distinct nodes and 3D bounds; maximum 125 nodes, 400 members.')
      return graph
    }
  }
}

export function createMainSolidWorkerHandler(post:(response:MainSolidResponse,transfer?:ArrayBuffer[])=>void) {
  return createWorkerHandler<MainSolidRequest,MainSolidResponse,MainSolidResults[keyof MainSolidResults]>(post,{
    validate:(value)=>{
      const request=value as Partial<MainSolidRequest>|null
      if(!request || request.version!==1 || !Number.isSafeInteger(request.id) || request.id!<1
        || !request.job || !['solidDistance','selfIntersection','faceContacts','boundaryAgreement','shellDistance','faceDistance','surfaceDistance','curveDistance','sketchSnaps','bodySnaps','faceSketch','bodyEdges','topology','curveDisplay','profileDisplay','surfaceMesh','surfaceBoundary','measureVertices','measureEdge','primitive','modelGraphImport','displayMesh','restoreDocument','brepTool','curveChainInspection','curveOffset','nurbsEdit','pointEdit','sketchEdit','boolean','sceneEdit','curveMatch','surfaceMatch','seamPrepare','surfaceBuild','nurbsRefit','profilePrepare','profileEdit','bodyEdit','revolve','extrusion','main','cad','inspect','meshContacts','truss','latticeGraph','structuralSections','bondedSolid'].includes(request.job.kind))return null
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

const instanceCache=new SolidInstanceBatchCache()
