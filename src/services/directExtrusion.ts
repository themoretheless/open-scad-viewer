import { extrudeSketchBrep, parseDirectDocument, type DirectBody, type DirectDocument, type DirectSketch } from './directModeling'
import { cross3, xyPlane, type SketchPlane } from './directSketchGeometry'
import { booleanNurbsBrep, extrudeBrepCurves, tessellateNurbsBrep, transformNurbsBrep, type NurbsBrep } from './geometry/brep'
import { authorBrepProfile, transformBrepProfile, validateBrepProfile } from './geometry/brepProfile'
import { booleanPolygonMeshes } from './geometry/polygon'
import { stringifyMeshJson } from './meshJson'

export function sameSketchPlane(a: SketchPlane = xyPlane(), b: SketchPlane = xyPlane()) {
  return [a.origin,a.u,a.v].every((p,i)=>p.every((v,k)=>Math.abs(v-[b.origin,b.u,b.v][i][k])<1e-7))
}
/** Nested contours use even/odd fill; circles remain rational curves. */
export function extrudeSketchProfile(sketches: readonly DirectSketch[], height: number, offset=0): NurbsBrep {
  if (!sketches.length || sketches.some(s=>!s.closed)) throw Error('Select at least one closed contour.')
  if (!Number.isFinite(height)||Math.abs(height)<.00001||!Number.isFinite(offset)) throw Error('Extrusion height must be finite and nonzero.')
  const plane=sketches[0].plane??xyPlane()
  if(sketches.some(s=>!sameSketchPlane(s.plane,plane))) throw Error('All profile contours must lie on the same workplane.')
  if(sketches.length===1)return extrudeSketchBrep(sketches[0],height,offset)
  const loops=sketches.flatMap(sketch=>{
    if(sketch.analytic?.kind==='circle') {
      const {center,radius}=sketch.analytic
      return transformBrepProfile(authorBrepProfile({kind:'circle',radius}),[1,0,0,0,0,1,0,0,0,0,1,0,center[0],center[1],0,1]).loops
    }
    return authorBrepProfile({kind:'polygon',rings:[sketch.points]}).loops
  })
  const profile=validateBrepProfile(loops,'even-odd')
  const solid=extrudeBrepCurves(profile.loops,offset+Math.min(0,height),offset+Math.max(0,height))
  const n=cross3(plane.u,plane.v)
  return transformNurbsBrep(solid,[
    [plane.u[0],plane.v[0],n[0],plane.origin[0]],
    [plane.u[1],plane.v[1],n[1],plane.origin[1]],
    [plane.u[2],plane.v[2],n[2],plane.origin[2]], [0,0,0,1],
  ])
}
export interface DirectExtrusionOptions { sketchIds: string[]; height: number; offset: number; operation: 'new'|'union'|'difference'; targetId: string; id: string; segments?: number }
/** Shared preview/commit path; the input document is never changed. */
export function buildDirectExtrusion(document: DirectDocument, options: DirectExtrusionOptions): DirectBody | null {
  const sketches=options.sketchIds.map(id=>{const s=document.sketches.find(s=>s.id===id);if(!s)throw Error('Profile contour no longer exists.');return s})
  const tool=extrudeSketchProfile(sketches,options.height,options.offset)
  const target=document.bodies.find(b=>b.id===options.targetId)
  if(options.operation!=='new'&&!target)throw Error('Select a target body.')
  let brep:NurbsBrep|undefined=tool
  if(options.operation!=='new'&&target?.brep)brep=booleanNurbsBrep(target.brep,tool,options.operation)
  if(!brep.bodies.length)return null
  const mesh=tessellateNurbsBrep(brep,options.segments??8)
  let positions:Float64Array=mesh.positions.slice(),indices:Uint32Array=mesh.indices.slice()
  if(options.operation!=='new'&&target&&!target.brep) {
    const result=booleanPolygonMeshes(target.mesh,{positions,indices},options.operation)
    positions=result.positions;indices=result.indices;brep=undefined
    if(!indices.length)return null
  }
  return {...(options.operation==='new'?{id:options.id,name:`${sketches[0].name} · 3D`}:target!),mesh:{positions,indices},...(brep?{brep}:{})}
}
export function applyDirectExtrusionProfile(document: DirectDocument, options: DirectExtrusionOptions): DirectDocument {
  const body=buildDirectExtrusion(document,options)
  const next={...document,bodies:options.operation==='new'?[...document.bodies,...(body?[body]:[])]:document.bodies.flatMap(b=>b.id===options.targetId?(body?[body]:[]):[b])}
  return parseDirectDocument(stringifyMeshJson(next))
}
