import {requirePolylineSketch} from './retainedSketchProfile'
import {applyDirectRevolve,type DirectRevolveOptions} from './directProfileTools'
import {parseDirectDocument,type DirectDocument,type DirectSketch,type DirectBody} from './directModeling'
import {xyPlane,cross3,type Vec3} from './directSketchGeometry'
import {booleanNurbsBrep,revolveBrepProfile,createFacetedBrepRevolve,tessellateNurbsBrep,type NurbsBrep} from './geometry/brep'
import {stringifyMeshJson} from './meshJson'
export interface SolidRevolveOptions extends DirectRevolveOptions {
 sketchId:string;geometry:'exact'|'faceted';operation:'new'|'union'|'difference';targetId:string;id:string;name:string;tessellation:number
}
function authoredRevolve(sketch:DirectSketch, options:SolidRevolveOptions) {
 requirePolylineSketch(sketch)
 const axis=options.axis,offset=options.offset
 const signed=sketch.points.map(point=>axis==='y'?point[0]-offset:point[1]-offset)
 if(signed.some(value=>Math.abs(value)>1e-7&&Math.sign(value)!==Math.sign(signed.find(v=>Math.abs(v)>1e-7)!)))throw Error('The revolve profile must stay on one side of its axis.')
 const sign=Math.sign(signed.find(value=>Math.abs(value)>1e-7)??1)
 let profile=sketch.points.map(point=>[Math.abs(axis==='y'?point[0]-offset:point[1]-offset),axis==='y'?point[1]:point[0]] as [number,number])
 const area=profile.reduce((sum,p,i)=>{const q=profile[(i+1)%profile.length];return sum+p[0]*q[1]-q[0]*p[1]},0)
 if(area<0)profile=profile.reverse()
 const brep=structuredClone(options.geometry==='exact'?revolveBrepProfile(profile,options.angle):createFacetedBrepRevolve(profile,options.segments)),plane=sketch.plane??xyPlane(),normal=cross3(plane.u,plane.v)
 const apply=(point:number[])=>axis==='y'
  ? plane.origin.map((value,i)=>value+plane.u[i]*(offset+sign*point[0])+plane.v[i]*point[2]-normal[i]*sign*point[1])
  : plane.origin.map((value,i)=>value+plane.u[i]*point[2]+plane.v[i]*(offset+sign*point[0])+normal[i]*sign*point[1])
 for(const vertex of brep.vertices)vertex.point=apply(vertex.point) as Vec3
 for(const edge of brep.edges)edge.curve.controlPoints=edge.curve.controlPoints.map(apply)
 for(const face of brep.faces)face.surface.controlPoints=face.surface.controlPoints.map(row=>row.map(apply))
 return brep
}

/** One result for both preview and commit, including empty Boolean results. */
export function applySolidRevolve(document:DirectDocument,options:SolidRevolveOptions):DirectDocument {
 const sketch=document.sketches.find(s=>s.id===options.sketchId)
 if(!sketch)throw Error('Select a sketch.')
 requirePolylineSketch(sketch)
 const target=document.bodies.find(b=>b.id===options.targetId)
 if(options.operation!=='new'&&!target)throw Error('Select the target body.')
 if(options.geometry==='exact'&&options.operation!=='new'&&!target?.brep)throw Error('Exact B-rep revolve combination requires an authored B-rep target.')
 const native=options.geometry==='exact'||Math.abs(options.angle)===360
 if(!native&&options.operation!=='new'&&target?.brep)throw Error('Partial faceted revolve cannot be combined with an authored B-rep.')
 if(!native||options.operation!=='new'&&!target?.brep)return applyDirectRevolve(document,sketch.id,options,options.operation,options.targetId,options.id)
 let brep=authoredRevolve(sketch,options)
 if(options.operation!=='new')brep=booleanNurbsBrep(target!.brep!,brep,options.operation)
 let body:DirectBody|null=null
 if(brep.bodies.length){
  const mesh=tessellateNurbsBrep(brep,options.tessellation)
  body={...(options.operation==='new'?{id:options.id,name:options.name}:target!),brep,mesh:{positions:mesh.positions,indices:mesh.indices}}
 }
 const next={...document,bodies:options.operation==='new'?[...document.bodies,...(body?[body]:[])]:document.bodies.flatMap(b=>b.id===options.targetId?(body?[body]:[]):[b])}
 return parseDirectDocument(stringifyMeshJson(next))
}
