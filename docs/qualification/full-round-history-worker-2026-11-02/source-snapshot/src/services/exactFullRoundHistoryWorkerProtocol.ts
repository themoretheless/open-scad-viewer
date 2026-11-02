import type {MeshData} from '../core/mesh'
import type {ExactFullRoundHistoryEdit,ExactHistoryBudget} from './exactFullRoundHistory'
import {hasSafeSuccessPayload} from './geometryWorkerProtocol/isGeometryEvaluationResultPayload'
import {isMeshData} from './geometryWorkerProtocol/isRecord'
export interface ExactHistoryWorkerRequest {
  type:'exact-history-edit/1'
  id:string
  mesh:MeshData
  edit:ExactFullRoundHistoryEdit
  segments:number
  budget?:ExactHistoryBudget
}
/** Bounded structural transport checks; native proof remains in the Worker. */
export function isHistoryMesh(value:unknown):value is MeshData {
  return hasSafeSuccessPayload({meshes:[value],warnings:[]}) && isMeshData(value)
}
export function isHistoryRequest(value:unknown):value is ExactHistoryWorkerRequest {
  if(!value||typeof value!=='object')return false
  const v=value as ExactHistoryWorkerRequest
  if(Object.keys(v).some(k=>!['type','id','mesh','edit','segments','budget'].includes(k))
    ||v.type!=='exact-history-edit/1'||typeof v.id!=='string'||v.id.length>128
    ||!Number.isInteger(v.segments)||v.segments<1||v.segments>32||!isHistoryMesh(v.mesh))return false
  if(!v.edit||typeof v.edit!=='object'||!['restore','undo','redo','branch','rebase-source','reselect'].includes(v.edit.action))return false
  try{return JSON.stringify({edit:v.edit,budget:v.budget}).length<=4*1024*1024}catch{return false}
}
