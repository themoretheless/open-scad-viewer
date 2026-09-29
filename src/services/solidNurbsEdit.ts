import {bridgeNurbsCurves} from './directDimensions'
import type {DirectDocument} from './directModeling'
import {insertNurbsKnot,elevateNurbsCurve,trimNurbsCurveAtPoint,trimNurbsCurveAtScreenPoint,type NurbsScreenProjection} from './nurbsCurve'
import {insertNurbsSurfaceKnot,elevateNurbsSurface,isoNurbsCurve,trimNurbsSurface} from './nurbsSurface'
import {extrudeNurbsCurve} from './nurbsConstructors'
import {nurbsCurveToSketch,tessellateSolidNurbsSurface} from './solidNurbs'
export type SolidNurbsEditOptions={id:string}&(
 | {kind:'bridge';otherId:string;endA:'start'|'end';endB:'start'|'end';tension:number;createdId:string}
 | {kind:'knot';axis:'curve'|'u'|'v';value:number}
 | {kind:'elevate';axis:'curve'|'u'|'v'}
 | {kind:'extrude'|'bake';createdId:string}
 | {kind:'iso';axis:'u'|'v';value:number;createdId:string}
 | {kind:'trim-screen-point';point:[number,number];matrix:NurbsScreenProjection;keep:'start'|'end';radius:number}
 | {kind:'trim-point';point:number[];keep:'start'|'end';maxDistance:number}
 | {kind:'trim';bounds:[number,number,number,number]})
/** Run the existing native operations on an isolated document snapshot. */
export function applySolidNurbsEdit(source:DirectDocument,p:SolidNurbsEditOptions):DirectDocument {
 const d=structuredClone(source),curve=d.curves?.find(c=>c.id===p.id),surface=d.surfaces?.find(s=>s.id===p.id)
 if(!curve&&!surface)throw Error('Select an existing NURBS curve or surface.')
 if('createdId' in p&&(!p.createdId||[...d.sketches,...d.bodies,...d.curves??[],...d.surfaces??[]].some(o=>o.id===p.createdId)))throw Error('A new NURBS result needs a unique identity.')
 if(p.kind==='bridge'){
  const other=d.curves?.find(c=>c.id===p.otherId)
  if(!curve||!other||curve.id===other.id)throw Error('Select two distinct NURBS curves.')
  const bridge={sourceA:curve.id,sourceB:other.id,endA:p.endA,endB:p.endB,tension:p.tension}
  d.curves!.push({id:p.createdId,name:'G2 bridge',bridge,curve:bridgeNurbsCurves(curve.curve,other.curve,p.endA,p.endB,p.tension)})
 }else if(p.kind==='trim-screen-point'&&curve){
  curve.curve=trimNurbsCurveAtScreenPoint(curve.curve,p.point,p.matrix,p.keep,p.radius).curve
 }else if(p.kind==='trim-point'&&curve){
  curve.curve=trimNurbsCurveAtPoint(curve.curve,p.point,p.keep,p.maxDistance).curve
 }else if(p.kind==='knot'||p.kind==='elevate'){
  if(p.axis==='curve'&&curve)curve.curve=p.kind==='knot'?insertNurbsKnot(curve.curve,p.value):elevateNurbsCurve(curve.curve,curve.curve.degree+1)
  else if(p.axis!=='curve'&&surface)surface.surface=p.kind==='knot'?insertNurbsSurfaceKnot(surface.surface,p.axis,p.value):elevateNurbsSurface(surface.surface,p.axis,(p.axis==='u'?surface.surface.degreeU:surface.surface.degreeV)+1)
  else throw Error('Select the NURBS object for this axis.')
 }else if(p.kind==='extrude'&&curve){
  (d.surfaces??=[]).push({id:p.createdId,name:`${curve.name} · extrude`,surface:extrudeNurbsCurve(curve.curve,[0,0,10]),segmentsU:16,segmentsV:8})
 }else if(p.kind==='iso'&&surface){
  (d.curves??=[]).push({id:p.createdId,name:`${surface.name} · iso ${p.axis.toUpperCase()}`,curve:isoNurbsCurve(surface.surface,p.axis,p.value)})
 }else if(p.kind==='trim'&&surface)surface.surface=trimNurbsSurface(surface.surface,p.bounds)
 else if(p.kind==='bake'){
  if(surface)d.bodies.push({id:p.createdId,name:`${surface.name} · baked mesh`,mesh:tessellateSolidNurbsSurface(surface)})
  else if(curve)d.sketches.push({...nurbsCurveToSketch(curve),id:p.createdId})
 }else throw Error('Select a compatible NURBS object.')
 return d
}
