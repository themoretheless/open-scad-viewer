import {validOffsetDeviation} from '../nurbsOffsetDeviation'
import {validOffsetContact} from './results/offsetContact'
import {validOffsetEnvelope} from './results/offsetEnvelope'
import {validMinkowskiResult} from './results/minkowski'
import {validG2RunoutResult} from '../geometricG2'
import type {AssemblyFrameExport} from '../assemblyFrameExport'
import type { CurveMeasurement, FaceDistanceResult, PointMeasurement, ShellDistanceResult } from '../solidMeasurements'
import type { DirectDocument } from '../directModeling'
import type { DisplayMesh } from '../solidDisplayCache'
import { finite, isNumericSequence, vector } from '../mainSolidProtocol'
import type { MainSolidExpectation, MainSolidResults } from '../mainSolidProtocol'
import type { NurbsCurve, NurbsCurveDistance } from '../nurbsCurve'
import type { NurbsSurfaceDistance } from '../nurbsSurface'
import type { PolygonMesh } from '../geometry/polygon'
import type { KernelManifoldMetrics, KernelManifoldRepair, KernelManifoldReport } from '../geometry/manifoldAnalysis'
import type { MeshBooleanResult, MeshRepairResult } from '../meshEditing'
import type { ImportedMesh } from '../meshImport'
import type { MeshConvertResult, MeshExportArtifact } from '../meshConvert'
import { convertedMeshFileName, MESH_IMPORT_FORMATS } from '../meshFormats'
import type { SolidBooleanResult } from '../solidBoolean'
import type { SolidBrepToolResult } from '../solidBrepTool'
import type { StructuralSections } from '../structuralSections'
import type { SurfaceBoundaryReport } from '../solidSurfaceDiagnostics'
import type { TrussResponse } from '../trussAnalysis'
import { isAxialScreeningRows, isStructuralProfileCatalog, isThermalStrengthResult, isTrussFieldMeshes, isTrussTopologyResponse, isValidatedPrintProfile } from '../mechanicsProtocol'
import { arrayOf, distanceWitnessConsistent, numericSequenceOf } from './mainSolidExpectation'
import { isBondedSolidResult } from '../bondedSolidProtocol'
import { isBoundaryConnectivity, isMaterialAudit } from '../structuralSectionsValidation'
import { isNominalLatticeGraph } from '../latticeGraphProtocol'
import { isSourceBodyRecordPayload } from '../sourceBodyArchive'
import { validBodySnapGeometry, validSketchSnapGeometry } from '../solidSnapProtocol'
import { validBoundaryAgreement } from '../solidBoundaryAgreement'
import { validContactQualification } from '../nurbsOffsetContactQualification'
import { validCurveOffsetDiagnostics } from '../curveOffsetDiagnostics'
import { validEnvelopeFit } from '../nurbsOffsetEnvelopeFit'
import { validFaceContacts } from '../solidFaceContacts'
import { validMaterial } from '../solidMaterialVolume'
import { validMaterialWall } from '../solidMaterialWall'
import { validSelfIntersection } from '../solidSelfIntersection'
import { validSolidDistance } from '../solidDistance'
import { validSolidTopology } from '../solidTopologyProtocol'
import { validSourceBody } from '../sourceBody'
import { validWholeWall } from '../solidWholeWall'
import { validateProfileIntersectionReport } from '../geometry/profileIntersections'
import { validProfileBridgeChordReport } from '../nurbsProfileBridgeChords'

function validRepairMesh(value:unknown,vertices?:number,triangles?:number):value is PolygonMesh {
  const mesh=value as PolygonMesh
  if(!mesh||!isNumericSequence(mesh.positions)||!isNumericSequence(mesh.indices)
    ||!mesh.positions.length||mesh.positions.length%3||!mesh.indices.length||mesh.indices.length%3)return false
  const vertexCount=mesh.positions.length/3,triangleCount=mesh.indices.length/3
  return (vertices===undefined||vertices===vertexCount)&&(triangles===undefined||triangles===triangleCount)
    &&numericSequenceOf(mesh.positions,mesh.positions.length,finite)
    &&numericSequenceOf(mesh.indices,mesh.indices.length,index=>Number.isSafeInteger(index)&&Number(index)>=0&&Number(index)<vertexCount)
}

function validManifoldReport(value:unknown,vertices:number,triangles:number):value is KernelManifoldReport {
  const report=value as KernelManifoldReport
  if(!report||report.vertexCount!==vertices||report.triangleCount!==triangles
    ||!Number.isSafeInteger(report.componentCount)||report.componentCount<1
    ||typeof report.isManifold!=='boolean'||typeof report.isManifoldWithBoundary!=='boolean')return false
  const indices=(view:unknown,max:number,pairs=false)=>isNumericSequence(view)&&(!pairs||view.length%2===0)
    &&numericSequenceOf(view,view.length,index=>Number.isSafeInteger(index)&&Number(index)>=0&&Number(index)<max)
  return indices(report.boundaryEdges,vertices,true)&&indices(report.nonManifoldEdges,vertices,true)
    &&indices(report.orientationEdges,vertices,true)&&indices(report.degenerateTriangles,triangles)
    &&indices(report.nonManifoldVertices,vertices)&&indices(report.isolatedVertices,vertices)
}

function validManifoldRepair(value:unknown,mesh:PolygonMesh,after:KernelManifoldReport):value is KernelManifoldRepair {
  const repair=value as KernelManifoldRepair
  if(!repair||!validRepairMesh(repair)||repair.mode!==1
    ||repair.positions.length!==mesh.positions.length||repair.indices.length!==mesh.indices.length
    ||repair.isManifold!==after.isManifold||repair.isManifoldWithBoundary!==after.isManifoldWithBoundary)return false
  const counts=[repair.weldedVertices,repair.removedDegenerateTriangles,repair.flippedTriangles,repair.splitVertices,repair.filledHoles,repair.filledTriangles]
  const equal=(a:ArrayLike<number>,b:ArrayLike<number>)=>{for(let index=0;index<a.length;index++)if(a[index]!==b[index])return false;return true}
  return counts.every(count=>Number.isSafeInteger(count)&&count>=0)
    &&equal(repair.positions,mesh.positions)&&equal(repair.indices,mesh.indices)
}

/** Admit the result for this request, not merely any object with a result field. */
export function mainSolidResult(job:MainSolidExpectation, value:unknown): boolean {
  if(job.kind==='sourceBodyRestore')return validSourceBody(job,value)
  if(job.kind==='profileBridgeChords')return validProfileBridgeChordReport(job,value)
  if (!value || typeof value!=='object') return false
  if(job.kind==='g2RunoutSurface')return validG2RunoutResult(value,job.maxCells,job.maxExactWork,job.maxPatches,job.tolerances)
if(job.kind==='minkowski3d')return validMinkowskiResult(job,value,document=>mainSolidResult({kind:'extrusion'},document))
if(job.kind==='animationFrames'){const v=value as AssemblyFrameExport;return v?.frames?.length>0&&v.frames.length<=300&&finite(v.fps)&&v.fps>0&&v.width===960&&v.height===640&&v.frames.every(f=>finite(f.time)&&typeof f.svg==='string'&&f.svg.startsWith('<svg'))}
if(job.kind==='animationSample'){const v=value as MainSolidResults['animationSample'];return v?.document?.version===1&&Array.isArray(v.document.bodies)&&Array.isArray(v.document.sketches)&&finite(v.camera?.yaw)&&finite(v.camera?.pitch)}
if(job.kind==='drawingPdf')return value instanceof Uint8Array&&value.length>0&&value.length<=32*1024*1024
if(job.kind==='drawingSheets')return Array.isArray(value)&&value.length>0&&value.length<=40&&value.every(s=>s&&typeof s.svg==='string'&&s.svg.length<32*1024*1024&&finite(s.width)&&finite(s.height)&&typeof s.title==='string')

  if(job.kind==='offsetContactQualification')return validContactQualification(job,value)
  if(job.kind==='offsetEnvelopeFit')return validEnvelopeFit(job,value)
  if(job.kind==='offsetEnvelope')return validOffsetEnvelope(job,value,mainSolidResult)
  if(job.kind==='offsetDeviation')return validOffsetDeviation(job.options,value)
  if(job.kind==='offsetContactTangent'||job.kind==='offsetSourceBoundary'||job.kind==='trimmedOffsetContactBand'||job.kind==='offsetContactBand')return validOffsetContact(job,value,mainSolidResult)
  if(job.kind==='wholeWall')return validWholeWall(job,value)
  if(job.kind==='materialWall')return validMaterialWall(job,value)
  if(job.kind==='materialSegment'||job.kind==='materialChord')return validMaterial(job,value)
  if(job.kind==='solidDistance')return validSolidDistance(job,value)
  if(job.kind==='selfIntersection')return validSelfIntersection(job,value)
  if(job.kind==='faceContacts')return validFaceContacts(job,value)
  if(job.kind==='boundaryAgreement')return validBoundaryAgreement(job,value)
  if(job.kind==='shellDistance'){
    const v=value as ShellDistanceResult
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    if(v.method!=='interval-trimmed-face-pairs'||v.scope!=='boundary-shells-bounded-joins'||v.containment!=='not-classified'
      ||v.toleranceMm!==job.toleranceMm||v.toleranceUv!==job.toleranceUv||v.maxCells!==job.maxCells||v.maxDomainCells!==job.maxDomainCells
      ||v.pairs!==job.domains[0].length*job.domains[1].length||!Number.isInteger(v.evaluatedPairs)||v.evaluatedPairs<0||v.evaluatedPairs>v.pairs
      ||!Number.isInteger(v.cells)||v.cells<0||v.cells>job.maxCells||!Number.isInteger(v.domainCells)||v.domainCells<0||v.domainCells>job.maxDomainCells
      ||!Array.isArray(v.distanceIntervalMm)||v.distanceIntervalMm.length!==2||!finite(v.distanceIntervalMm[0])||v.distanceIntervalMm[0]<0||typeof v.converged!=='boolean')return false
    const [lo,hi]=v.distanceIntervalMm
    if(hi===null){if(v.faces!==null||v.parameters!==null||v.points!==null||v.pointEnclosures!==null||v.converged)return false}
    else if(!finite(hi)||hi<lo||!arrayOf(v.faces,2,f=>Number.isInteger(f)&&Number(f)>=0)
      ||!v.faces!.every((f,i)=>!!job.domains[i][f])
      ||!arrayOf(v.parameters,2,uv=>arrayOf(uv,2,finite))
      ||!v.parameters!.every((uv,i)=>uv.every((t,k)=>t>=job.domains[i][v.faces![i]][k][0]&&t<=job.domains[i][v.faces![i]][k][1]))
      ||!arrayOf(v.points,2,vector)||!arrayOf(v.pointEnclosures,2,p=>arrayOf(p,3,interval))||v.evaluatedPairs===0||v.cells===0||v.domainCells===0||!distanceWitnessConsistent(v.points!,v.pointEnclosures!,lo,hi))return false
    return v.converged?v.reason==='tolerance'&&hi!==null&&hi-lo<=job.toleranceMm
      :['work-limit','domain-work-limit','pair-resolution-limit','empty-domain'].includes(v.reason)&&(hi===null||hi-lo>job.toleranceMm)&&(v.reason!=='empty-domain'||hi===null)
  }
  if(job.kind==='faceDistance'){
    const v=value as FaceDistanceResult
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    if(v.method!=='interval-trimmed-surface-subdivision'||v.scope!=='trimmed-surfaces-bounded-joins'
      ||v.toleranceMm!==job.toleranceMm||v.toleranceUv!==job.toleranceUv||v.maxCells!==job.maxCells||v.maxDomainCells!==job.maxDomainCells
      ||!Number.isInteger(v.cells)||v.cells<1||v.cells>job.maxCells||!Number.isInteger(v.domainCells)||v.domainCells<1||v.domainCells>job.maxDomainCells
      ||!Array.isArray(v.distanceIntervalMm)||v.distanceIntervalMm.length!==2||!finite(v.distanceIntervalMm[0])||v.distanceIntervalMm[0]<0||typeof v.converged!=='boolean')return false
    const [lo,hi]=v.distanceIntervalMm
    if(hi===null){if(v.parameters!==null||v.points!==null||v.pointEnclosures!==null||v.converged)return false}
    else if(!finite(hi)||hi<lo||!arrayOf(v.parameters,2,uv=>arrayOf(uv,2,finite))
      ||!v.parameters!.every((uv,i)=>uv.every((t,k)=>!!job.domains[i][k]&&t>=job.domains[i][k][0]&&t<=job.domains[i][k][1]))
      ||!arrayOf(v.points,2,vector)||!arrayOf(v.pointEnclosures,2,p=>arrayOf(p,3,interval))||!distanceWitnessConsistent(v.points!,v.pointEnclosures!,lo,hi))return false
    return v.converged?v.reason==='tolerance'&&hi!==null&&hi-lo<=job.toleranceMm
      :['work-limit','domain-work-limit','precision-limit','empty-domain'].includes(v.reason)&&(hi===null||hi-lo>job.toleranceMm)&&(v.reason!=='empty-domain'||hi===null)
  }
  if(job.kind==='surfaceDistance'){
    const v=value as NurbsSurfaceDistance
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    return v.method==='interval-tensor-de-boor-pair-subdivision'&&v.scope==='untrimmed-surfaces'
      &&v.toleranceMm===job.toleranceMm&&v.maxCells===job.maxCells
      &&Number.isInteger(v.cells)&&v.cells>=1&&v.cells<=job.maxCells
      &&interval(v.distanceIntervalMm)&&v.distanceIntervalMm[0]>=0
      &&arrayOf(v.parameters,2,uv=>arrayOf(uv,2,finite))&&v.parameters.every((uv,i)=>uv.every((t,k)=>t>=job.domains[i][k][0]&&t<=job.domains[i][k][1]))
      &&arrayOf(v.points,2,vector)&&arrayOf(v.pointEnclosures,2,p=>arrayOf(p,3,interval))
      &&distanceWitnessConsistent(v.points,v.pointEnclosures,v.distanceIntervalMm[0],v.distanceIntervalMm[1])
      &&typeof v.converged==='boolean'
      &&(v.converged?v.reason==='tolerance'&&v.distanceIntervalMm[1]-v.distanceIntervalMm[0]<=job.toleranceMm
        :['work-limit','precision-limit'].includes(v.reason)&&v.distanceIntervalMm[1]-v.distanceIntervalMm[0]>job.toleranceMm)
  }
  if(job.kind==='curveDistance'){
    const v=value as NurbsCurveDistance
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    return [2,3].includes(job.dimension)&&v.method==='interval-de-boor-pair-subdivision'
      &&v.toleranceMm===job.toleranceMm&&v.maxCells===job.maxCells
      &&Number.isInteger(v.cells)&&v.cells>=1&&v.cells<=job.maxCells
      &&interval(v.distanceIntervalMm)&&v.distanceIntervalMm[0]>=0
      &&arrayOf(v.parameters,2,finite)&&v.parameters.every((t,i)=>t>=job.domains[i][0]&&t<=job.domains[i][1])
      &&arrayOf(v.points,2,p=>arrayOf(p,job.dimension,finite))
      &&arrayOf(v.pointEnclosures,2,p=>arrayOf(p,job.dimension,interval))
      &&typeof v.converged==='boolean'
      &&(v.converged?v.reason==='tolerance'&&v.distanceIntervalMm[1]-v.distanceIntervalMm[0]<=job.toleranceMm
        :['work-limit','precision-limit'].includes(v.reason)&&v.distanceIntervalMm[1]-v.distanceIntervalMm[0]>job.toleranceMm)
  }
  if(job.kind==='sketchSnaps')return validSketchSnapGeometry(value,job.maxPoints,job.maxSegments,job.maxCircles)
  if(job.kind==='bodySnaps')return validBodySnapGeometry(value,job.maxPoints,job.maxSegments)
  if(job.kind==='faceSketch'){const v=value as MainSolidResults['faceSketch'];return !!v.plane&&vector(v.plane.origin)&&vector(v.plane.u)&&vector(v.plane.v)&&vector(v.normal)&&Array.isArray(v.outline)&&v.outline.length>0&&v.outline.every(loop=>Array.isArray(loop)&&loop.length>=3&&loop.every(p=>arrayOf(p,2,finite)))}
  if(job.kind==='bodyEdges')return Array.isArray(value)&&(job.exact?value.length===job.edges:value.length<=job.edges)&&value.every((e,i)=>e&&e.index===i&&typeof e.id==='string'&&e.id.length>0&&[e.a,e.b].every(n=>Number.isSafeInteger(n)&&n>=0&&n<job.vertices)&&Array.isArray(e.points)&&e.points.length>=2&&e.points.every(vector))
  if(job.kind==='topology')return validSolidTopology(value,job.vertices,job.triangles)
  if(job.kind==='curveChainInspection')return validCurveOffsetDiagnostics(value,job.segments)&&value.checks<=job.maxPairs
  if(job.kind==='curveDisplay')return [2,3].includes(job.dimension)&&Array.isArray(value)&&value.length===49&&value.every(p=>Array.isArray(p)&&p.length===job.dimension&&p.every(finite))
  if(job.kind==='profileDisplay')return Array.isArray(value)&&value.length===job.counts.length&&value.every((loop,i)=>Array.isArray(loop)&&loop.length===job.counts[i]&&loop.every(p=>Array.isArray(p)&&p.length===2&&p.every(finite)))
  if(job.kind==='surfaceMesh'){
    const m=value as PolygonMesh
    if(![job.u,job.v].every(n=>Number.isInteger(n)&&n>=2&&n<=64)||!isNumericSequence(m.positions)||!isNumericSequence(m.indices))return false
    // The native mesher triangulates each untrimmed grid cell around its center.
    const vertices=m.positions.length/3,triangles=m.indices.length/3
    return Number.isInteger(vertices)&&vertices>0&&vertices<=(job.u+1)*(job.v+1)+job.u*job.v&&Number.isInteger(triangles)&&triangles<=job.u*job.v*4
      &&numericSequenceOf(m.positions,m.positions.length,finite)&&numericSequenceOf(m.indices,m.indices.length,i=>Number.isSafeInteger(i)&&Number(i)>=0&&Number(i)<vertices)
  }
  if(job.kind==='surfaceBoundary'){
    const v=value as SurfaceBoundaryReport,angle=(a:unknown)=>a===null||finite(a)&&a>=0&&a<=90
    return Number.isInteger(job.samples)&&job.samples>=2&&job.samples<=257&&v.samplingOnly===true
      &&finite(v.maxGapMm)&&v.maxGapMm>=0&&angle(v.maxTangentPlaneAngleDeg)
      &&Number.isInteger(v.undefinedNormals)&&v.undefinedNormals>=0&&v.undefinedNormals<=job.samples
      &&Number.isInteger(v.worstGapSample)&&v.worstGapSample>=0&&v.worstGapSample<job.samples&&typeof v.sampledWithinTolerance==='boolean'
      &&arrayOf(v.samples,job.samples,item=>{const s=item as SurfaceBoundaryReport['samples'][number];return !!s&&finite(s.t)&&s.t>=0&&s.t<=1&&vector(s.a)&&vector(s.b)&&finite(s.gapMm)&&s.gapMm>=0&&angle(s.tangentPlaneAngleDeg)})
  }
  if(job.kind==='measureVertices'){
    const v=value as PointMeasurement
    return vector(v.a)&&vector(v.b)&&vector(v.deltaMm)&&finite(v.distanceMm)&&v.distanceMm>=0
  }
  if(job.kind==='manifoldMetrics'){
    const v=value as KernelManifoldMetrics
    const count=(n:unknown)=>Number.isSafeInteger(n)&&Number(n)>=0
    const integer=(n:unknown)=>Number.isSafeInteger(n)
    return [v.vertexCount,v.triangleCount,v.edgeCount,v.componentCount].every(count)
      &&integer(v.eulerCharacteristic)&&finite(v.signedVolume)&&finite(v.surfaceArea)&&v.surfaceArea>=0
      &&typeof v.watertight==='boolean'&&Array.isArray(v.components)&&v.components.length===v.componentCount
      &&v.components.every(c=>[c.vertexCount,c.triangleCount,c.edgeCount,c.boundaryEdges].every(count)
        &&integer(c.eulerCharacteristic)&&(c.genus===null||count(c.genus))
        &&finite(c.signedVolume)&&finite(c.surfaceArea)&&c.surfaceArea>=0)
  }
  if(job.kind==='meshManifold'){
    const entries=value as MainSolidResults['meshManifold']
    return Array.isArray(entries)&&entries.length===job.objects.length&&entries.every((entry,index)=>{
      const object=job.objects[index]
      return !!object&&entry?.id===object.id&&validManifoldReport(entry.report,object.vertices,object.triangles)
    })
  }
  if(job.kind==='meshRepair'){
    const result=value as MeshRepairResult,vertices=result.mesh?.positions.length/3,triangles=result.mesh?.indices.length/3
    return validRepairMesh(result.mesh)&&validManifoldReport(result.before,job.vertices,job.triangles)
      &&validManifoldReport(result.after,vertices,triangles)
      &&(result.repair===null?result.before.isManifold&&result.after.isManifold:validManifoldRepair(result.repair,result.mesh,result.after))
  }
  if(job.kind==='meshBoolean'){
    const result=value as MeshBooleanResult,report=result.report,triangles=result.mesh?.indices.length/3
    const count=(n:unknown)=>Number.isSafeInteger(n)&&Number(n)>=0
    return validRepairMesh(result.mesh)&&!!report&&report.operation===job.operation
      &&arrayOf(report.operandRepaired,2,value=>typeof value==='boolean')
      &&finite(report.toleranceMm)&&report.toleranceMm>=0&&count(report.work)&&count(report.fragments)
      &&report.weldEpsilonMm===job.weldEpsilonMm&&report.relativeTolerance===job.relativeTolerance
      &&arrayOf(report.inputTriangles,2,count)&&report.inputTriangles.every((count,index)=>count===job.triangles[index])
      &&report.outputTriangles===triangles&&typeof report.isManifold==='boolean'
  }
  if(job.kind==='meshImport'){
    const result=value as ImportedMesh,manifold=result.manifold,before=manifold?.before,after=manifold?.after,repair=manifold?.repair
    const count=(n:unknown)=>Number.isSafeInteger(n)&&Number(n)>=0
    const vertices=result.positions?.length/3,triangles=result.indices?.length/3
    if(!validRepairMesh(result)||!manifold||manifold.mode!==job.repair||!before||!after
      ||!(MESH_IMPORT_FORMATS as readonly string[]).includes(result.format)
      ||result.vertexCount!==vertices||result.triangleCount!==triangles
      ||!count(result.sourceVertexCount)||result.sourceVertexCount<before.vertexCount||!count(result.degenerateTriangles)
      ||!validManifoldReport(before,before.vertexCount,before.triangleCount)||!validManifoldReport(after,vertices,triangles))return false
    if(repair===null)return job.repair==='none'||before.isManifold
    const counts=[repair.weldedVertices,repair.removedDegenerateTriangles,repair.flippedTriangles,repair.splitVertices,repair.filledHoles,repair.filledTriangles]
    return job.repair!=='none'&&!before.isManifold&&repair.mode===(job.repair==='full'?1:0)&&counts.every(count)
      &&repair.isManifold===after.isManifold&&repair.isManifoldWithBoundary===after.isManifoldWithBoundary
  }
  if(job.kind==='meshConvert'){
    const result=value as MeshConvertResult,source=result.source
    const extension=job.target==='stl_binary'?'stl':job.target
    const count=(n:unknown)=>Number.isSafeInteger(n)&&Number(n)>=0
    return result.data instanceof Uint8Array&&result.data.byteLength>0
      &&typeof result.mimeType==='string'&&result.mimeType.length>0
      &&result.format===job.target&&result.extension===extension
      &&result.fileName===convertedMeshFileName(job.fileName,extension)
      &&source.fileName===job.fileName&&source.byteLength===job.byteLength
      &&(MESH_IMPORT_FORMATS as readonly string[]).includes(source.format)
      &&[source.triangleCount,source.vertexCount,source.sourceVertexCount,source.degenerateTriangles].every(count)
  }
  if(job.kind==='meshExport'){
    const result=value as MeshExportArtifact
    const extension=job.target==='stl_binary'?'stl':job.target
    return result.data instanceof Uint8Array&&result.data.byteLength>0
      &&typeof result.mimeType==='string'&&result.mimeType.length>0
      &&result.extension===extension
  }
  if(job.kind==='measureEdge'){
    const v=value as CurveMeasurement
    return vector(v.point)&&finite(v.parameter)&&v.parameter>=0&&v.parameter<=1&&finite(v.curvaturePerMm)&&v.curvaturePerMm>=0
      &&(v.radiusMm===null||finite(v.radiusMm)&&v.radiusMm>0)
  }
  if(job.kind==='partialAnnularPreview') {
    const v=value as MainSolidResults['partialAnnularPreview'], q=v.evidence?.qualification, mesh=v.body?.mesh
    if(v.body?.id!==job.id || !job.bodyIdentity || v.body.brep?.topologyIds?.bodies.length!==1
      || v.body.brep.topologyIds.bodies[0]!==job.bodyIdentity || !mesh
      || q?.status!=='preview-only' || q.commitAllowed!==false
      || q.boundaryIntersectionProof!=='unqualified' || q.transitionContinuityProof!=='unqualified'
      || !isNumericSequence(mesh.positions)||!isNumericSequence(mesh.indices)
      || !mesh.positions.length || mesh.positions.length%3 || !mesh.indices.length || mesh.indices.length%3)return false
    return numericSequenceOf(mesh.positions,mesh.positions.length,finite)
      && numericSequenceOf(mesh.indices,mesh.indices.length,i=>Number.isSafeInteger(i)&&Number(i)>=0&&Number(i)<mesh.positions.length/3)
  }
  if(job.kind==='displayMesh'){
    const v=value as DisplayMesh,m=v.mesh
    if(!m||!isNumericSequence(m.positions)||!isNumericSequence(m.indices)||m.positions.length%3||m.indices.length%3)return false
    const count=m.indices.length/3,vertices=m.positions.length/3
    return numericSequenceOf(m.positions,m.positions.length,finite)
      &&numericSequenceOf(m.indices,m.indices.length,i=>Number.isSafeInteger(i)&&Number(i)>=0&&Number(i)<vertices)
      &&(m.uv===undefined||numericSequenceOf(m.uv,vertices*2,finite))
      &&arrayOf(v.normals,count,vector)
      &&(v.closed==null||arrayOf(v.closed,count,x=>typeof x==='boolean'))
      &&(v.workClosed==null||arrayOf(v.workClosed,job.triangles,x=>typeof x==='boolean'))
      &&(v.map===null?count===job.triangles:count<=4000&&arrayOf(v.map,count,i=>Number.isSafeInteger(i)&&Number(i)>=0&&Number(i)<job.triangles))
  }
  if(job.kind==='brepTool'){
    const v=value as SolidBrepToolResult
    if(v.id!==job.id||v.kind!==job.operation)return false
    if(v.kind==='display'){
      const d=v.diagnostics
      return !!d&&!!d.report&&['boundaryEdges','nonManifoldEdges','orientationConflicts','degenerateTriangles'].every(k=>Number.isSafeInteger(d.report[k as keyof typeof d.report])&&Number(d.report[k as keyof typeof d.report])>=0)
        &&Array.isArray(d.defectLines)&&d.defectLines.every(l=>typeof l.kind==='string'&&Array.isArray(l.points)&&l.points.every(vector))
        &&!!d.section&&vector(d.section.normal)&&finite(d.section.offsetMm)&&Array.isArray(d.section.contours)
        &&d.section.contours.every(c=>Array.isArray(c.points)&&c.points.every(vector)&&Array.isArray(c.sourceTriangles))
        &&Array.isArray(d.section.collapsedSegmentTriangles)&&typeof d.sectionError==='string'&&typeof d.boundaryError==='string'
    }
    if(v.kind==='mesh')return mainSolidResult({kind:'extrusion'},v.document)&&v.document.bodies.some(b=>b.id===v.id)
    if(v.kind!=='mass'||!v.mass)return false
    const m=v.mass
    return finite(m.surfaceAreaMm2)&&m.surfaceAreaMm2>=0&&finite(m.signedVolumeMm3)&&vector(m.centroid)
      &&arrayOf(m.inertiaMm5,3,vector)&&arrayOf(m.conservativeBounds,2,vector)
      &&finite(m.areaErrorEstimateMm2)&&m.areaErrorEstimateMm2>=0&&finite(m.volumeErrorEstimateMm3)&&m.volumeErrorEstimateMm3>=0
      &&Number.isSafeInteger(m.evaluations)&&m.evaluations>=0&&m.status==='converged_estimate'&&m.solidGeometryStatus==='not_certified'
  }
  if(job.kind==='boolean') {
    const v=value as SolidBooleanResult
    return mainSolidResult({kind:'extrusion'},v.document)&&typeof v.exact==='boolean'&&finite(v.toleranceMm)&&v.toleranceMm>=0
      &&(v.resultId===null||typeof v.resultId==='string'&&v.document.bodies.some(b=>b.id===v.resultId))
  }
  if(job.kind==='curveMatch'||job.kind==='surfaceMatch'||job.kind==='seamPrepare') {
    const v=value as MainSolidResults['curveMatch'|'surfaceMatch'|'seamPrepare'],r=v.report
    if(!mainSolidResult({kind:'extrusion'},v.document)||!r||typeof r.accepted!=='boolean'||typeof r.reason!=='string'||r.accepted!==(r.reason==='accepted'))return false
    const nonnegative=(n:unknown)=>finite(n)&&n>=0
    if(job.kind==='curveMatch'){
      const c=(v as MainSolidResults['curveMatch']).report
      return c.method==='outward-endpoint-handle-wedge'&&nonnegative(c.positionErrorUpper)&&nonnegative(c.maxAngleDegrees)
        &&(c.sineAngleUpper===null||nonnegative(c.sineAngleUpper))&&(c.angleDegreesUpper===null||nonnegative(c.angleDegreesUpper))
        &&typeof c.regularityCertified==='boolean'&&typeof c.orientationCertified==='boolean'
        &&(!c.accepted||c.regularityCertified&&c.orientationCertified&&c.angleDegreesUpper!==null&&c.angleDegreesUpper<=c.maxAngleDegrees)
    }
    if(job.kind==='surfaceMatch'){
      const c=(v as MainSolidResults['surfaceMatch']).report,b=c.errorBounds
      return [1,2].includes(c.continuityOrder)&&finite(c.normalScale)&&c.normalScale>0&&nonnegative(c.errorUpper)&&nonnegative(c.maxError)
        &&c.method==='homogeneous-normalized-boundary-jets'&&c.seamBasis==='exact-affine-knot-correspondence'&&!!b&&b.wholeSeam===true&&b.normalizedParameters===true
        &&nonnegative(b.positionUpper)&&nonnegative(b.firstDerivativeUpper)&&(b.secondDerivativeUpper===null||nonnegative(b.secondDerivativeUpper))&&(b.mixedDerivativeUpper===null||nonnegative(b.mixedDerivativeUpper))
        &&(!c.accepted||c.regularityCertified&&c.tangentialSmoothnessCertified&&c.errorUpper<=c.maxError)
    }
    const c=(v as MainSolidResults['seamPrepare']).report,b=(v as MainSolidResults['seamPrepare']).basis
    return nonnegative(c.budget)&&!!b&&Number.isInteger(b.degree)&&b.degree>=1&&Number.isInteger(b.controlCount)&&b.controlCount>b.degree
      &&b.normalizedSeam===true&&typeof b.editedReversed==='boolean'&&arrayOf(b.referenceDomain,2,finite)&&arrayOf(b.editedDomain,2,finite)
      &&(!c.accepted||c.wholeSurface===true&&nonnegative(c.referenceErrorUpper)&&nonnegative(c.editedErrorUpper)&&c.referenceErrorUpper!<=c.budget&&c.editedErrorUpper!<=c.budget)
  }
  if(job.kind==='trimmedCurveOffset') {
    const v=value as MainSolidResults['trimmedCurveOffset'],r=v.report
    if(!mainSolidResult({kind:'extrusion'},v.document)||!r||r.accepted!==true||r.method!=='represented-bevel-offset-fill/1'
      ||r.fillRule!==job.fillRule||r.regionTrimmed!==true||r.topologyScope!=='represented-reconstructed-chord-graph'
      ||r.regionTopologyCertified!==false||r.originalOffsetTopologyCertified!==false
      ||!finite(r.sourceWireErrorUpperMm)||r.sourceWireErrorUpperMm<0||r.sourceWireErrorUpperMm>job.toleranceMm
      ||!finite(r.intersectionConstructionErrorUpperMm)||r.intersectionConstructionErrorUpperMm<0||r.intersectionConstructionErrorUpperMm>job.intersectionToleranceMm
      ||!Array.isArray(v.loopIds)||v.loopIds.length>128)return false
    const used=new Set<string>()
    return v.loopIds.every(ids=>{
      if(!Array.isArray(ids)||!ids.length||ids.length>128)return false
      const curves=[] as NurbsCurve[]
      for(const id of ids){
        if(typeof id!=='string'||!id.startsWith(job.createdId+':')||used.has(id))return false
        const item=v.document.curves?.find(c=>c.id===id)
        if(!item||item.curve.degree!==1||item.curve.periodic===true)return false
        used.add(id);curves.push(item.curve)
      }
      const equal=(a:number[],b:number[])=>a.length===b.length&&a.every((x,i)=>x===b[i])
      for(let i=1;i<curves.length;i++)if(!equal(curves[i-1]!.controlPoints.at(-1)!,curves[i]!.controlPoints[0]!))return false
      return equal(curves[0]!.controlPoints[0]!,curves.at(-1)!.controlPoints.at(-1)!)
    })
  }
  if(job.kind==='curveOffset') {
    const nonnegative=(n:unknown):n is number=>finite(n)&&n>=0
    const v=value as MainSolidResults['curveOffset'],r=v.report
    return mainSolidResult({kind:'extrusion'},v.document)&&!!r&&r.accepted===true
      &&(r.method==='outward-rational-jets-chord-bound/1'?r.wholeCurve===true:r.method==='outward-source-offset-bevel-wire/1'&&r.wholeCurve===false&&r.wholeWire===true&&r.regionTrimmed===false)&&typeof r.closed==='boolean'
      &&r.regionTopologyCertified===false&&r.offsetRegularityCertified===false
      &&nonnegative(r.errorUpperMm)&&finite(r.toleranceMm)&&r.toleranceMm>0&&r.errorUpperMm<=r.toleranceMm
      &&Array.isArray(r.cells)&&r.cells.length<=65536&&r.cells.every(c=>!!c&&arrayOf(c.domain,2,finite)
        &&c.domain[0]<c.domain[1]&&nonnegative(c.errorUpperMm)&&c.errorUpperMm<=r.errorUpperMm)
      &&(r.wholeWire!==true||r.cells.length>0&&r.cells.every((c,i)=>{
        const role=c.source
        return c.domain[0]===i&&c.domain[1]===i+1&&!!role&&(role.kind==='source-offset'
          ?arrayOf(role.domain,2,finite)&&role.domain[0]<role.domain[1]
          :role.kind==='bevel'&&finite(role.sourceKnot))
      }))
      &&(r.cells.length?validCurveOffsetDiagnostics(r.chainDiagnostics,r.cells.length):r.chainDiagnostics===null)
  }
  if(job.kind==='surfaceBuild') {
    const v=value as MainSolidResults['surfaceBuild'],r=v.report
    if(job.runoutId&&v.document){
      const patches=(v.document.surfaces??[]).filter(s=>s.id===job.runoutId||s.id.startsWith(job.runoutId+':')).map(s=>s.surface)
      if(!validG2RunoutResult({patches,report:v.runoutReport},100000,1000000,256)||!v.runoutReport?.withinTolerance)return false
    }
    return typeof v.error==='string'&&(v.document===null?v.error.length>0:v.error===''&&mainSolidResult({kind:'extrusion'},v.document))
      &&(r===null||!!r&&typeof r.accepted==='boolean'&&finite(r.sampledControlDeviation)&&r.sampledControlDeviation>=0&&finite(r.budget)&&r.budget>=0
        &&Number.isInteger(r.stations)&&r.stations>0&&Number.isInteger(r.sections)&&r.sections>0&&typeof r.continuousBound==='boolean'&&r.method==='double-reflection-fourfold-section-refinement'
        &&(r.continuousCertificate===undefined?r.continuousBound===false&&r.accepted===(r.sampledControlDeviation<=r.budget)
          :!!r.continuousCertificate&&r.continuousCertificate.scope==='matched-parameter-profile-deviation'
            &&typeof r.continuousCertificate.method==='string'&&(r.continuousCertificate.reason===null||typeof r.continuousCertificate.reason==='string')
            &&r.continuousCertificate.regularityCertified===false&&r.continuousCertificate.globalEmbeddingCertified===false&&r.continuousCertificate.seamSmoothnessCertified===false
            &&Number.isInteger(r.continuousCertificate.cells)&&r.continuousCertificate.cells>=0
            &&Number.isInteger(r.continuousCertificate.maxCells)&&r.continuousCertificate.maxCells>=r.continuousCertificate.cells&&r.continuousCertificate.maxCells<=100000
            &&r.continuousBound===(r.continuousCertificate.errorUpper!==null)
            &&(r.continuousCertificate.errorUpper===null||finite(r.continuousCertificate.errorUpper)&&r.continuousCertificate.errorUpper>=0)
            &&r.continuousCertificate.withinBudget===(r.continuousBound&&r.continuousCertificate.errorUpper!<=r.budget)
            &&r.accepted===r.continuousCertificate.withinBudget)
        &&(!r.seamContinuity||!['G1','G2'].includes(r.seamContinuity)||!!r.seamCertificate
          &&r.seamCertificate.certified===true&&r.seamCertificate.exact===true&&r.seamCertificate.regularityCertified===true
          &&r.seamCertificate.order===(r.seamContinuity==='G2'?2:1)
          &&r.seamCertificate.scope==='represented-closing-strip-jets'&&r.seamCertificate.method==='dyadic-cubic-closing-strips-exact-predicate'
          &&Number.isInteger(r.seamCertificate.work)&&r.seamCertificate.work>=0&&r.seamCertificate.work<=1000000)&&(!v.document||r.accepted))
  }
  if(job.kind==='nurbsRefit') {
    const v=value as MainSolidResults['nurbsRefit'],c=v.certificate
    return mainSolidResult({kind:'extrusion'},v.document)&&!!c&&/^nurbs-foundation\/[1-5]$/.test(c.version)
      &&typeof c.accepted==='boolean'&&typeof c.rolledBack==='boolean'&&!(c.accepted&&c.rolledBack)
      &&['geometryErrorUpper','hausdorffErrorUpper','dataSiteErrorUpper','budget'].every(key=>{const n=c[key as keyof typeof c];return n===undefined||finite(n)&&n>=0})
      &&!!c.evidence&&!!c.evidence.toleranceIdentity&&typeof c.evidence.toleranceIdentity.canonical==='string'
  }
  if(job.kind==='profileIntersections') {
    try {validateProfileIntersectionReport(value,job.loops,job.options);return true} catch {return false}
  }
  if(job.kind==='profilePrepare') {
    const v=value as MainSolidResults['profilePrepare'],r=v.report
    const point=(p:unknown)=>arrayOf(p,2,finite)
    if(r?.diagnosticLoops!==undefined){
      if(r.accepted||r.reason!=='invalid-contour'||!Array.isArray(r.diagnosticLoops)||r.diagnosticLoops.length!==1||
        !Array.isArray(r.diagnosticLoops[0])||r.diagnosticLoops[0].length<2||r.diagnosticLoops[0].length>254||
        !r.curveSources||r.curveSources.length!==r.diagnosticLoops[0].length)return false
      if(r.intersections){try{validateProfileIntersectionReport(r.intersections,r.diagnosticLoops,{toleranceMm:1e-7,maxPairs:128,maxBoxes:8192})}catch{return false}}
      else if(typeof r.intersectionDiagnosticError!=='string'||!r.intersectionDiagnosticError)return false
    }else if(r?.intersections!==undefined||r?.intersectionDiagnosticError!==undefined||r?.diagnosticDisplay!==undefined)return false
    if(r?.diagnosticDisplay!==undefined&&(!r.intersections||!Array.isArray(r.diagnosticDisplay)||r.diagnosticDisplay.length>254||
      r.diagnosticDisplay.some(s=>!s||!Number.isInteger(s.curve)||s.curve<0||s.curve>=r.diagnosticLoops![0].length||
        !['intersection','overlap','unproven'].includes(s.kind)||!Array.isArray(s.points)||s.points.length<2||s.points.length>49||!s.points.every(point))||new Set(r.diagnosticDisplay.map(s=>s.curve)).size!==r.diagnosticDisplay.length))return false
    return mainSolidResult({kind:'extrusion'},v.document)&&typeof v.id==='string'&&!!v.plane&&vector(v.plane.origin)&&vector(v.plane.u)&&vector(v.plane.v)
      &&!!r&&typeof r.accepted==='boolean'&&['accepted','endpoint-topology','disconnected','invalid-contour'].includes(r.reason)
      &&(r.projectionMaxDeviationMm===undefined||finite(r.projectionMaxDeviationMm)&&r.projectionMaxDeviationMm>=0&&r.projectionMaxDeviationMm<=1e-7)
      &&r.accepted===(r.reason==='accepted')&&Array.isArray(r.points)&&r.points.every(point)
      &&Array.isArray(r.defects)&&r.defects.every(d=>d&&Number.isInteger(d.chain)&&d.chain>=0&&['start','end'].includes(d.end)&&point(d.point)&&['gap','ambiguous'].includes(d.kind)&&Array.isArray(d.candidates)&&d.candidates.every(i=>Number.isInteger(i)&&i>=0))
      &&Array.isArray(r.connectors)&&r.connectors.every(c=>c&&point(c.a)&&point(c.b))
      &&(r.curveSources===undefined||Array.isArray(r.curveSources)&&r.curveSources.length<=512&&r.curveSources.every(s=>s&&Number.isInteger(s.chain)&&s.chain>=0&&typeof s.reversed==='boolean'&&(!job.chainSegments||s.chain<job.chainSegments.length)
        &&(s.connector===false?Number.isInteger(s.segment)&&s.segment>=0&&(!job.chainSegments||s.segment<job.chainSegments[s.chain]):s.connector===true&&['start','end'].includes(s.end)&&Number.isInteger(s.nextChain)&&s.nextChain>=0&&(!job.chainSegments||s.nextChain<job.chainSegments.length)))
        &&(!r.profile||Array.isArray(r.profile.loops)&&r.profile.loops.every(Array.isArray)&&r.curveSources.length===r.profile.loops.reduce((n,loop)=>n+loop.length,0)))
      &&(!r.segmentDefect||(['intersection','overlap','degenerate','unproven'].includes(r.segmentDefect.kind)&&Array.isArray(r.segmentDefect.segments)&&r.segmentDefect.segments.every(s=>Number.isInteger(s.index)&&s.index>=0&&point(s.a)&&point(s.b))))
  }
  if(job.kind==='meshContacts') {
    const v=value as MainSolidResults['meshContacts']
    if(v.scope!=='display-mesh-all-contacts'||v.relativeTolerance!==1e-9||typeof v.complete!=='boolean'
      ||v.maxWork!==job.maxWork||v.maxContacts!==job.maxContacts||!Number.isInteger(v.work)||v.work<0||v.work>job.maxWork
      ||(v.complete?v.stopReason!==null:!['work-limit','contact-limit'].includes(v.stopReason??''))
      ||!Array.isArray(v.contacts)||v.contacts.length>job.maxContacts||!Array.isArray(v.triangleIds)||!Array.isArray(v.lines))return false
    if(!v.contacts.every((c,i)=>!!c&&arrayOf(c.triangles,2,t=>Number.isInteger(t)&&Number(t)>=0&&Number(t)<job.triangles)
      &&c.triangles[0]<c.triangles[1]&&vector(c.point)&&finite(c.toleranceMm)&&c.toleranceMm>0
      &&Number.isInteger(c.sharedVertices)&&c.sharedVertices>=0&&c.sharedVertices<=3
      &&(i===0||v.contacts[i-1].triangles[0]<c.triangles[0]||v.contacts[i-1].triangles[0]===c.triangles[0]&&v.contacts[i-1].triangles[1]<c.triangles[1])))return false
    const ids=[...new Set(v.contacts.flatMap(c=>c.triangles))].sort((a,b)=>a-b)
    return JSON.stringify(v.contact)===JSON.stringify(v.contacts[0]??null)
      &&v.triangleIds.length===ids.length&&v.triangleIds.every((id,i)=>id===ids[i])
      &&arrayOf(v.lines,ids.length,line=>arrayOf(line,4,vector))
  }
  if (job.kind==='bondedSolid')return isBondedSolidResult(value,job.nodes,job.tets,job.bonds)
  if (job.kind==='latticeGraph') return isNominalLatticeGraph(value)
  if (job.kind==='structuralSections') {
    const v=value as StructuralSections
    const planeAxes={x:[1,2],y:[2,0],z:[0,1]}[job.axis]
    return v.modelKind==='finished-mesh-sections-v1'&&v.axis===job.axis
      &&arrayOf(v.planeAxes,2,n=>Number.isInteger(n)&&Number(n)>=0&&Number(n)<3)
      &&v.planeAxes.every((axis,i)=>axis===planeAxes[i])
      &&(v.selfIntersections==='not-checked'||v.selfIntersections==='checked-at-tolerance')
      &&!!v.sourceMesh&&isNumericSequence(v.sourceMesh.positions)&&v.sourceMesh.positions.length<=900000
      &&v.sourceMesh.positions.length%3===0&&numericSequenceOf(v.sourceMesh.positions,v.sourceMesh.positions.length,finite)
      &&isNumericSequence(v.sourceMesh.indices)&&v.sourceMesh.indices.length===v.triangleCount*3
      &&numericSequenceOf(v.sourceMesh.indices,v.sourceMesh.indices.length,n=>Number.isInteger(n)&&Number(n)>=0&&Number(n)<v.sourceMesh.positions.length/3)
      &&finite(v.volumeMm3)&&v.volumeMm3>0&&Number.isInteger(v.triangleCount)&&v.triangleCount>0&&v.triangleCount<=100000
      &&isBoundaryConnectivity(v.connectivity,v.triangleCount,v.sourceMesh.positions.length/3)
      &&isMaterialAudit(v)
      &&Array.isArray(v.sections)&&v.sections.length===job.stations.length&&v.sections.length>0&&v.sections.length<=64
      &&Array.from(v.sections).every((s,i)=>s&&s.positionMm===job.stations[i]&&finite(s.positionMm)&&typeof s.material==='boolean'&&Array.isArray(s.contours)
        &&(s.material?!!s.properties&&finite(s.properties.areaMm2)&&s.properties.areaMm2>0
          &&arrayOf(s.properties.centroidMm,2,finite)&&finite(s.properties.iuuMm4)&&s.properties.iuuMm4>0
          &&finite(s.properties.ivvMm4)&&s.properties.ivvMm4>0&&finite(s.properties.iuvMm4):s.properties===null))
  }
  if(job.kind==='trussField')return isTrussFieldMeshes(value,job.members)
  if(job.kind==='trussScreening')return isAxialScreeningRows(value,job.members,job.responses)
  if(job.kind==='printStrengthProfile')return isValidatedPrintProfile(value,job.profile)
  if(job.kind==='thermalStrength')return isThermalStrengthResult(value)
  if(job.kind==='trussTopology')return isTrussTopologyResponse(value,job.nodes,job.members,job.cases)
  if(job.kind==='structuralProfiles')return isStructuralProfileCatalog(value)
  if (job.kind==='truss') {
    const v=value as TrussResponse, {nodes,members}=job
    return nodes>0 && nodes<=125 && members>0 && members<=400
      && arrayOf(v.displacementsMm,nodes,vector)
      && arrayOf(v.reactionsN,nodes,vector)
      && arrayOf(v.axialForcesN,members,finite)
      && arrayOf(v.axialStressesMpa,members,finite)
      && finite(v.maxDeflectionMm) && v.maxDeflectionMm>=0
      && finite(v.maxRelativeResidual) && v.maxRelativeResidual>=0 && v.maxRelativeResidual<=1e-9
      && Number.isInteger(v.freeDofs) && v.freeDofs>=0 && v.freeDofs<=3*nodes
  }
  if (job.kind==='inspect') return Array.isArray(value) && value.every(v => v
    && typeof v.a==='string' && typeof v.b==='string' && finite(v.overlapMm3) && finite(v.gapMm)
    && (v.closestPoints===undefined || v.closestPoints===null || arrayOf(v.closestPoints,2,vector))
    && (v.displayMeshOnly===undefined || v.displayMeshOnly===true))
  const v=value as DirectDocument
  return v.version===1 && Array.isArray(v.sketches) && Array.isArray(v.bodies)
    && (v.sourceBodies===undefined||(Array.isArray(v.sourceBodies)&&v.sourceBodies.length<=200&&v.sourceBodies.every(isSourceBodyRecordPayload)))
}
