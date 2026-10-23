import {parseDirectDocument,type DirectDocument} from './directModeling'
import {combineSketchProfiles} from './retainedSketchProfile'
import {offsetSketch,extendSketch,sampleCurve,xyPlane} from './directSketchGeometry'
import {stringifyMeshJson} from './meshJson'
export type SolidProfileEditOperation='profile-union'|'profile-difference'|'profile-intersection'|'offset'|'extend'|'curve'
export interface SolidProfileEditOptions {
 operation:SolidProfileEditOperation;id:string;inputs:string[];distance:number;end:'start'|'end';cx:number;cy:number;radius:number;start:number;sweep:number
}
export function isSolidProfileEdit(operation:unknown):operation is SolidProfileEditOperation {
 return ['profile-union','profile-difference','profile-intersection','offset','extend','curve'].includes(String(operation))
}
/** Produce a complete candidate document without mutating the worker request. */
export function applySolidProfileEdit(document:DirectDocument,p:SolidProfileEditOptions):DirectDocument {
 const d=parseDirectDocument(stringifyMeshJson(document))
 if(p.operation.startsWith('profile-')){
  const sketches=p.inputs.map(id=>{const sketch=d.sketches.find(s=>s.id===id);if(!sketch)throw Error('Profile input no longer exists.');return sketch})
  const operation=p.operation==='profile-union'?'union':p.operation==='profile-difference'?'difference':'intersection'
  const result=combineSketchProfiles(sketches,operation)
  d.sketches=d.sketches.filter(s=>!p.inputs.includes(s.id)).concat(result)
 }else{
  const index=d.sketches.findIndex(s=>s.id===p.id),sketch=d.sketches[index]
  if(!sketch)throw Error('Select an existing sketch.')
  if(p.operation==='offset')d.sketches[index]=offsetSketch(sketch,p.distance)
  else if(p.operation==='extend'){
   const plane=sketch.plane??xyPlane()
   d.sketches[index]=extendSketch(sketch,p.end,d.sketches.filter(s=>{const other=s.plane??xyPlane();return [plane.origin,plane.u,plane.v].every((a,i)=>a.every((x,j)=>Math.abs(x-[other.origin,other.u,other.v][i][j])<1e-7))}))
  }else if(p.operation==='curve'){
   if(!sketch.analytic)throw Error('Select an analytic circle or arc.')
   sketch.analytic={...sketch.analytic,center:[p.cx,p.cy],radius:p.radius,start:p.start,sweep:p.sweep}
   sketch.points=sampleCurve(sketch.analytic);delete sketch.dimensions
  }else throw Error('Unsupported profile edit.')
 }
 return parseDirectDocument(stringifyMeshJson(d))
}
