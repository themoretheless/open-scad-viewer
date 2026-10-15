import {createG2AnnularTube} from './geometricG2'
import {parseDirectDocument,type DirectDocument} from './directModeling'
import {stringifyMeshJson} from './meshJson'
import {createBrepBox,createBrepCylinder,createBrepFrustum,createBrepSphere,createBrepTorus,createBrepTube,createFacetedBrepCylinder,createFacetedBrepSphere,extrudeBrepPolygon,tessellateNurbsBrep,type NurbsBrep} from './geometry/brep'
import {revolvePolygonProfile} from './geometry/polygon'
export type SolidPrimitiveKind='box'|'wedge'|'cylinder'|'frustum'|'tube'|'cone'|'sphere'|'torus'
export interface SolidPrimitiveOptions {kind:SolidPrimitiveKind;id:string;name:string;size:number;topRadius:number;innerRadius:number;round:'exact'|'faceted';segments:number;group?:string;g2TransitionSize?:number;g2TransitionSizes?:[number,number,number,number]}
/** Native constructors author exact geometry; no provisional extrusion/revolve is needed. */
export function addSolidPrimitive(document:DirectDocument,o:SolidPrimitiveOptions):DirectDocument {
 if(!Number.isFinite(o.size)||o.size<.1||o.size>10000)throw Error('Size must be 0.1–10000 mm')
 if(o.round!=='exact'&&o.round!=='faceted')throw Error('Invalid round geometry mode')
 if(o.g2TransitionSize!==undefined&&(o.kind!=='tube'||!Number.isFinite(o.g2TransitionSize)||o.g2TransitionSize<=0))throw Error('G2 tube transition size must be positive and finite')
 if(o.g2TransitionSizes&&(o.kind!=='tube'||o.g2TransitionSize===undefined||o.g2TransitionSizes.length!==4||o.g2TransitionSizes.some(x=>!Number.isFinite(x)||x<=0)))throw Error('G2 tube transition sizes must be positive and finite')
 const r=o.size/2
 let brep:NurbsBrep|undefined
 switch(o.kind){
  case 'box':brep=createBrepBox([-r,-r,0],[r,r,o.size]);break
  case 'wedge':brep=extrudeBrepPolygon([[-r,-r],[r,-r],[-r,r]],0,o.size);break
  case 'cylinder':brep=o.round==='exact'?createBrepCylinder(r,o.size):createFacetedBrepCylinder(r,o.size,48);break
  case 'frustum':brep=createBrepFrustum(r,o.topRadius,o.size);break
  case 'tube':brep=o.g2TransitionSize===undefined?createBrepTube(r,o.innerRadius,o.size):createG2AnnularTube(r,o.innerRadius,o.size,o.g2TransitionSize,o.g2TransitionSizes).model;break
  case 'cone':if(o.round==='exact')brep=createBrepFrustum(r,0,o.size);break
  case 'sphere':brep=o.round==='exact'?createBrepSphere(r):createFacetedBrepSphere(r,16,8);break
  case 'torus':brep=createBrepTorus(r,o.topRadius);break
  default:throw Error('Unknown solid primitive')
 }
 const built=brep?tessellateNurbsBrep(brep,o.segments):revolvePolygonProfile([[0,0],[r,0],[0,o.size]],360,48,true)
 const body={id:o.id,name:o.name,mesh:{positions:built.positions,indices:built.indices},...(brep?{brep}:{}),...(o.group?{group:o.group}:{})}
 return parseDirectDocument(stringifyMeshJson({...document,bodies:[...document.bodies,body]}))
}
