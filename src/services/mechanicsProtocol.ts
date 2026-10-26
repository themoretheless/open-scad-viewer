import type {MeshData} from '../core/mesh'
import type {StructuralProfileFamily,StructuralProfileProperties} from './structuralProfiles'
import type {TrussTopologyResponse} from './trussAnalysis'
import type {AxialScreeningRow} from './trussScreening'
import {isMeshData} from './geometryWorkerProtocol/isRecord'

const finite=(value:unknown):value is number=>typeof value==='number'&&Number.isFinite(value)
const positive=(value:unknown)=>finite(value)&&value>0
const arrayOf=(value:unknown,length:number,check:(item:unknown)=>boolean)=>
  Array.isArray(value)&&value.length===length&&value.every(check)
const families=new Set<StructuralProfileFamily>(['ipe','heb','w','gost-8239','sto-aschm-b','sto-aschm-sh','sto-aschm-k'])

/** Strict worker-boundary validation for one catalog entry with full properties. */
export function isStructuralProfileProperties(value:unknown):value is StructuralProfileProperties {
  const profile=value as StructuralProfileProperties
  return !!profile&&typeof profile==='object'
    &&typeof profile.designation==='string'&&profile.designation.length>0&&profile.designation.length<=64
    &&families.has(profile.family)
    &&(['heightMm','flangeWidthMm','webThicknessMm','flangeThicknessMm','areaMm2','massKgM',
      'iStrongMm4','iWeakMm4','wStrongMm3','wWeakMm3','jMm4','cwMm6'] as const)
      .every(key=>positive(profile[key]))
}

export function isStructuralProfileCatalog(value:unknown):value is StructuralProfileProperties[] {
  if(!Array.isArray(value)||value.length<1||value.length>128||!value.every(isStructuralProfileProperties))return false
  const designations=value.map(profile=>profile.designation)
  return new Set(designations).size===designations.length
}

/** Validate dimensions and every numeric array against the originating request. */
export function isTrussTopologyResponse(value:unknown,nodes:number,members:number,cases:number):value is TrussTopologyResponse {
  const result=value as TrussTopologyResponse
  if(!result||typeof result!=='object'||nodes<1||nodes>125||members<1||members>400||cases<1||cases>32)return false
  if(!arrayOf(result.densities,members,item=>finite(item)&&item>=1e-4&&item<=1)
    ||!arrayOf(result.areasMm2,members,positive)
    ||!arrayOf(result.compliancesNmm,cases,item=>finite(item)&&item>=0)
    ||!arrayOf(result.axialForcesN,cases,forces=>arrayOf(forces,members,finite)))return false
  return positive(result.volumeMm3)&&positive(result.targetVolumeMm3)
    &&finite(result.initialComplianceNmm)&&result.initialComplianceNmm>=0
    &&finite(result.complianceNmm)&&result.complianceNmm>=0
    &&Number.isInteger(result.iterations)&&result.iterations>=0&&result.iterations<=1000
    &&typeof result.converged==='boolean'
    &&finite(result.maxDensityChange)&&result.maxDensityChange>=0&&result.maxDensityChange<=1
    &&Array.isArray(result.activeMembers)&&result.activeMembers.length<=members
    &&result.activeMembers.every((index,i)=>Number.isInteger(index)&&index>=0&&index<members
      &&(i===0||result.activeMembers[i-1]<index)&&result.densities[index]>=0.5)
}

export function isTrussFieldMeshes(value:unknown,members:number):value is MeshData[] {
  if(!Number.isInteger(members)||members<1||members>400||!Array.isArray(value)||value.length<1||value.length>3)return false
  const colors=new Set<string>()
  let triangles=0
  for(const mesh of value){
    if(!isMeshData(mesh)||mesh.indices.length===0)return false
    const color=JSON.stringify(mesh.color)
    if(colors.has(color))return false
    colors.add(color);triangles+=mesh.indices.length/3
  }
  return triangles<=members*8
}

export function isAxialScreeningRows(value:unknown,members:number,responses:number):value is AxialScreeningRow[] {
  if(!Number.isInteger(members)||members<1||members>400||!Number.isInteger(responses)||responses<1||responses>32
    ||!Array.isArray(value)||value.length!==members)return false
  const indices=new Set<number>()
  return value.every(row=>{
    const item=row as AxialScreeningRow
    if(!item||typeof item!=='object'||!Number.isInteger(item.memberIndex)||item.memberIndex<0||item.memberIndex>=members
      ||indices.has(item.memberIndex)||!finite(item.utilization)||item.utilization<0
      ||!Number.isInteger(item.governingResponse)||item.governingResponse<0||item.governingResponse>=responses
      ||!finite(item.stressMpa))return false
    indices.add(item.memberIndex)
    return item.status==='overloaded'?item.utilization>1
      :item.status==='low-demand'?item.utilization<.3
      :item.status==='within-axial-limit'&&item.utilization>=.3&&item.utilization<=1
  })
}
