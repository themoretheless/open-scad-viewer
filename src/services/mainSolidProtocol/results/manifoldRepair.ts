import type {PolygonMesh} from '../../geometry/polygon'
import type {KernelManifoldRepair,KernelManifoldReport} from '../../geometry/manifoldAnalysis'
import {finite,isNumericSequence} from '../../mainSolidProtocol'
import {numericSequenceOf} from '../mainSolidExpectation'

export function validRepairMesh(value:unknown,vertices?:number,triangles?:number):value is PolygonMesh {
  const mesh=value as PolygonMesh
  if(!mesh||!isNumericSequence(mesh.positions)||!isNumericSequence(mesh.indices)
    ||!mesh.positions.length||mesh.positions.length%3||!mesh.indices.length||mesh.indices.length%3)return false
  const vertexCount=mesh.positions.length/3,triangleCount=mesh.indices.length/3
  return (vertices===undefined||vertices===vertexCount)&&(triangles===undefined||triangles===triangleCount)
    &&numericSequenceOf(mesh.positions,mesh.positions.length,finite)
    &&numericSequenceOf(mesh.indices,mesh.indices.length,index=>Number.isSafeInteger(index)&&Number(index)>=0&&Number(index)<vertexCount)
}

export function validManifoldReport(value:unknown,vertices:number,triangles:number):value is KernelManifoldReport {
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

export function validManifoldRepair(value:unknown,mesh:PolygonMesh,after:KernelManifoldReport):value is KernelManifoldRepair {
  const repair=value as KernelManifoldRepair
  if(!repair||!validRepairMesh(repair)||repair.mode!==1
    ||repair.positions.length!==mesh.positions.length||repair.indices.length!==mesh.indices.length
    ||repair.isManifold!==after.isManifold||repair.isManifoldWithBoundary!==after.isManifoldWithBoundary)return false
  const counts=[repair.weldedVertices,repair.removedDegenerateTriangles,repair.flippedTriangles,repair.splitVertices,repair.filledHoles,repair.filledTriangles]
  const equal=(a:ArrayLike<number>,b:ArrayLike<number>)=>{for(let index=0;index<a.length;index++)if(a[index]!==b[index])return false;return true}
  return counts.every(count=>Number.isSafeInteger(count)&&count>=0)
    &&equal(repair.positions,mesh.positions)&&equal(repair.indices,mesh.indices)
}
