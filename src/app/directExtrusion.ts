import {sketchProfile} from './retainedSketchProfile'
import { DirectInputError } from '../i18n/kernelErrors'
import { extrudeSketchBrep, parseDirectDocument, type DirectBody, type DirectDocument, type DirectSketch } from './directModeling'
import { cross3, xyPlane, type SketchPlane } from './directSketchGeometry'
import { booleanNurbsBrep, extrudeBrepCurves, tessellateNurbsBrep, transformNurbsBrep, type NurbsBrep } from '../kernel/geometry/brep'
import { authorBrepProfile, transformBrepProfile, validateBrepProfile } from '../kernel/geometry/brepProfile'
import { booleanPolygonMeshes } from '../kernel/geometry/polygon'
import { stringifyMeshJson } from '../exchange/meshJson'

export function sameSketchPlane(a: SketchPlane = xyPlane(), b: SketchPlane = xyPlane()) {
  const ao = a.origin, au = a.u, av = a.v
  const bo = b.origin, bu = b.u, bv = b.v
  return Math.abs(ao[0] - bo[0]) < 1e-7 && Math.abs(ao[1] - bo[1]) < 1e-7 && Math.abs(ao[2] - bo[2]) < 1e-7
    && Math.abs(au[0] - bu[0]) < 1e-7 && Math.abs(au[1] - bu[1]) < 1e-7 && Math.abs(au[2] - bu[2]) < 1e-7
    && Math.abs(av[0] - bv[0]) < 1e-7 && Math.abs(av[1] - bv[1]) < 1e-7 && Math.abs(av[2] - bv[2]) < 1e-7
}
/** True when two non-adjacent edges of a closed polygon cross or touch. */
export function polygonSelfIntersects(points: readonly (readonly number[])[]): boolean {
  const n=points.length
  if(n<4||n>4096)return false
  const orient=(a:readonly number[],b:readonly number[],c:readonly number[])=>(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
  const eps=1e-12
  for(let i=0;i<n;i++) {
    const a=points[i],b=points[(i+1)%n]
    for(let j=i+2;j<n;j++) {
      if(i===0&&j===n-1)continue
      const c=points[j],d=points[(j+1)%n]
      const o1=orient(a,b,c),o2=orient(a,b,d),o3=orient(c,d,a),o4=orient(c,d,b)
      if(((o1>eps&&o2<-eps)||(o1<-eps&&o2>eps))&&((o3>eps&&o4<-eps)||(o3<-eps&&o4>eps)))return true
    }
  }
  return false
}
/** Nested contours use even/odd fill; circles remain rational curves. */
export function extrudeSketchProfile(sketches: readonly DirectSketch[], height: number, offset=0): NurbsBrep {
  if (!sketches.length || sketches.some(s=>!s.closed)) throw new DirectInputError('DIRECT_PROFILE_MISSING','Select at least one closed contour.',undefined,sketches.filter(s=>!s.closed).map(s=>s.id))
  if (!Number.isFinite(height)||Math.abs(height)<.00001||!Number.isFinite(offset)) throw new DirectInputError('DIRECT_EXTRUDE_HEIGHT','Extrusion height must be finite and nonzero.','height')
  const plane=sketches[0].plane??xyPlane()
  if(sketches.some(s=>!sameSketchPlane(s.plane,plane))) throw new DirectInputError('DIRECT_PROFILE_PLANE','All profile contours must lie on the same workplane.',undefined,sketches.filter(s=>!sameSketchPlane(s.plane,plane)).map(s=>s.id))
  for(const sketch of sketches)if(!sketch.retainedProfile&&!sketch.analytic&&polygonSelfIntersects(sketch.points))
    throw new DirectInputError('DIRECT_SKETCH_SELF_INTERSECTION','Profile contour intersects itself.',undefined,[sketch.id])
  if(sketches.length===1&&!sketches[0].retainedProfile)return extrudeSketchBrep(sketches[0],height,offset)
  const loops=sketches.flatMap(sketch=>{
    if(sketch.retainedProfile)return sketchProfile(sketch).loops
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
export type ExtrudeUpToSide = 'near'|'far'
/**
 * C9 "build up to selected geometry": the signed extrusion height that brings the profile plane
 * (shifted by `offset` along its normal) to the near or far side of a body, measured on the body's
 * mesh vertices along the plane normal. Exact for planar faces; curved faces use their tessellation.
 * A body that crosses the profile plane is reached at its extent on the normal side.
 */
export function extrudeHeightUpToBody(plane: SketchPlane|undefined, offset: number, body: Pick<DirectBody,'id'|'mesh'>, side: ExtrudeUpToSide): number {
  const p=plane??xyPlane(), n=cross3(p.u,p.v), len=Math.hypot(n[0],n[1],n[2])
  const pos=body.mesh.positions
  if(!(len>0)||pos.length<3)throw new DirectInputError('DIRECT_TARGET_BODY','Select a target body.',undefined,[body.id])
  const base=p.origin[0]*n[0]/len+p.origin[1]*n[1]/len+p.origin[2]*n[2]/len+offset
  let min=Infinity,max=-Infinity
  for(let i=0;i+2<pos.length;i+=3){const d=(pos[i]*n[0]+pos[i+1]*n[1]+pos[i+2]*n[2])/len-base;if(d<min)min=d;if(d>max)max=d}
  const eps=1e-7
  const height=max<=eps&&min<-eps?(side==='near'?max:min):min>=-eps?(side==='near'?min:max):max
  if(!Number.isFinite(height)||Math.abs(height)<.00001)throw new DirectInputError('DIRECT_EXTRUDE_HEIGHT','Extrusion height must be finite and nonzero.','height')
  return height
}
/** A target plane for C9 "extrude up to": a point on it and its normal (any length). */
export interface ExtrudeUpToPlane { origin: readonly number[]; normal: readonly number[] }
/**
 * C9 "build up to a selected face or plane": the signed height along the profile normal at which
 * the profile plane (shifted by `offset`) meets the target plane, measured at the profile origin.
 * Refuses (DIRECT_UP_TO_PLANE) a target parallel to the profile normal and one that coincides with
 * the shifted profile plane.
 */
export function extrudeHeightUpToPlane(plane: SketchPlane|undefined, offset: number, target: ExtrudeUpToPlane): number {
  const p=plane??xyPlane(), n=cross3(p.u,p.v), len=Math.hypot(n[0],n[1],n[2])
  const m=target.normal, mlen=Math.hypot(m[0],m[1],m[2])
  if(!(len>0)||!(mlen>0)||!Number.isFinite(mlen)||!Number.isFinite(offset))throw new DirectInputError('DIRECT_UP_TO_PLANE','The up-to plane needs a nonzero normal.')
  const base=[0,1,2].map(i=>p.origin[i]+n[i]/len*offset)
  const denom=(n[0]*m[0]+n[1]*m[1]+n[2]*m[2])/len/mlen
  if(Math.abs(denom)<1e-9)throw new DirectInputError('DIRECT_UP_TO_PLANE','The up-to plane runs parallel to the profile normal and is never reached.')
  const height=((target.origin[0]-base[0])*m[0]+(target.origin[1]-base[1])*m[1]+(target.origin[2]-base[2])*m[2])/mlen/denom
  if(!Number.isFinite(height))throw new DirectInputError('DIRECT_UP_TO_PLANE','The up-to plane cannot be reached.')
  if(Math.abs(height)<.00001)throw new DirectInputError('DIRECT_UP_TO_PLANE','The up-to plane coincides with the profile plane: the height would be zero.')
  return height
}
/** One planar face of a body that "extrude up to" can target. */
export interface PlanarFaceTarget extends ExtrudeUpToPlane { bodyId: string; face: number }
/**
 * Planar faces of a body. A B-rep body lists its faces whose control net is flat (curved faces are
 * left out, never approximated by their tessellation); a mesh-only body lists coplanar triangle groups.
 */
export function planarFaceTargets(body: Pick<DirectBody,'id'|'mesh'|'brep'>, limit=64): PlanarFaceTarget[] {
  const out: PlanarFaceTarget[]=[]
  if(body.brep){
    const tol=Math.max(body.brep.toleranceMm*10,1e-6)
    body.brep.faces.forEach((face,i)=>{
      if(out.length>=limit)return
      const plane=brepFacePlane(face.surface.controlPoints,tol)
      if(plane)out.push({bodyId:body.id,face:i,...plane})
    })
    return out
  }
  const pos=body.mesh.positions, idx=body.mesh.indices, groups=new Map<string,PlanarFaceTarget>()
  for(let t=0;t+2<idx.length&&groups.size<limit;t+=3){
    const a=[0,1,2].map(k=>pos[idx[t]*3+k]),b=[0,1,2].map(k=>pos[idx[t+1]*3+k]),c=[0,1,2].map(k=>pos[idx[t+2]*3+k])
    const nn=cross3([b[0]-a[0],b[1]-a[1],b[2]-a[2]],[c[0]-a[0],c[1]-a[1],c[2]-a[2]]),l=Math.hypot(nn[0],nn[1],nn[2])
    if(!(l>1e-12))continue
    const u=nn.map(x=>x/l),d=u[0]*a[0]+u[1]*a[1]+u[2]*a[2]
    const key=[...u.map(x=>Math.round(x*1e5)),Math.round(d*1e4)].join(',')
    if(!groups.has(key))groups.set(key,{bodyId:body.id,face:groups.size,origin:a,normal:u})
  }
  return [...groups.values()]
}
/**
 * C9: the planar face target under a picked working-mesh triangle, or null when the triangle lies on a
 * curved face (its tangent plane is never offered in place of the face) or the index is out of range.
 */
export function planarFaceTargetAtTriangle(body: Pick<DirectBody,'id'|'mesh'|'brep'>, triangle: number): PlanarFaceTarget|null {
  const pos=body.mesh.positions, idx=body.mesh.indices
  if(!Number.isInteger(triangle)||triangle<0||triangle*3+2>=idx.length)return null
  const pts=[0,1,2].map(k=>{const v=idx[triangle*3+k];return [pos[v*3],pos[v*3+1],pos[v*3+2]]})
  const n=cross3([pts[1][0]-pts[0][0],pts[1][1]-pts[0][1],pts[1][2]-pts[0][2]],[pts[2][0]-pts[0][0],pts[2][1]-pts[0][1],pts[2][2]-pts[0][2]]),l=Math.hypot(n[0],n[1],n[2])
  if(!(l>1e-12))return null
  const size=Math.max(1,...pts.flat().map(Math.abs)), tol=Math.max((body.brep?.toleranceMm??0)*10,1e-5*size)
  for(const f of planarFaceTargets(body,Infinity)){
    const fl=Math.hypot(f.normal[0],f.normal[1],f.normal[2])
    if(!(fl>0)||Math.abs(Math.abs(n[0]*f.normal[0]+n[1]*f.normal[1]+n[2]*f.normal[2])/(l*fl)-1)>1e-4)continue
    if(pts.every(p=>Math.abs((p[0]-f.origin[0])*f.normal[0]+(p[1]-f.origin[1])*f.normal[1]+(p[2]-f.origin[2])*f.normal[2])/fl<=tol))return f
  }
  return null
}
/** The plane through a flat control net, or null when the net is curved. */
export function brepFacePlane(controlPoints: readonly (readonly (readonly number[])[])[], tolerance=1e-6): ExtrudeUpToPlane|null {
  const pts=controlPoints.flat()
  if(pts.length<3)return null
  const a=pts[0]
  let normal:number[]|null=null
  for(let i=1;i<pts.length&&!normal;i++)for(let j=i+1;j<pts.length;j++){
    const nn=cross3([pts[i][0]-a[0],pts[i][1]-a[1],pts[i][2]-a[2]],[pts[j][0]-a[0],pts[j][1]-a[1],pts[j][2]-a[2]]),l=Math.hypot(nn[0],nn[1],nn[2])
    if(l>1e-9){normal=nn.map(x=>x/l);break}
  }
  if(!normal)return null
  for(const q of pts)if(Math.abs((q[0]-a[0])*normal[0]+(q[1]-a[1])*normal[1]+(q[2]-a[2])*normal[2])>tolerance)return null
  return {origin:[a[0],a[1],a[2]],normal}
}
export type DirectExtrusionOperation = 'new'|'union'|'difference'|'intersection'
/**
 * `targetIds` lists further bodies the same tool combines with in one command, after `targetId`,
 * so one cutter can subtract from several parts. Each target is combined independently.
 */
export interface DirectExtrusionOptions { sketchIds: string[]; height: number; offset: number; operation: DirectExtrusionOperation; targetId: string; targetIds?: string[]; id: string; segments?: number }
/** Ordered, de-duplicated combination targets; empty for a new body. */
export function directExtrusionTargets(options: Pick<DirectExtrusionOptions,'operation'|'targetId'|'targetIds'>): string[] {
  if(options.operation==='new')return []
  const ids=[options.targetId,...(options.targetIds??[])].filter(id=>id)
  const duplicates=ids.filter((id,i)=>ids.indexOf(id)!==i)
  if(duplicates.length)throw new DirectInputError('DIRECT_TARGET_BODY','Each target body may be selected only once.',undefined,[...new Set(duplicates)])
  if(!ids.length)throw new DirectInputError('DIRECT_TARGET_BODY','Select a target body.')
  return ids
}
/**
 * C9 preview: swaps every combination target in the scene for its previewed result, so extra
 * `targetIds` change in the viewport before Apply. A target missing from the result (consumed
 * by a difference or an empty intersection) disappears; bodies that are not targets stay as is.
 */
export function previewTargetBodies<B extends {id:string}>(scene:readonly B[],targetIds:readonly string[],result:readonly B[]|null|undefined,primary?:{id:string;body:B|null}):B[] {
  const replaced=new Map<string,B|null>()
  if(result)for(const id of targetIds)replaced.set(id,result.find(b=>b.id===id)??null)
  if(primary)replaced.set(primary.id,primary.body)
  if(!replaced.size)return [...scene]
  return scene.flatMap(b=>{if(!replaced.has(b.id))return [b];const next=replaced.get(b.id);return next?[next]:[]})
}
/** Shared preview/commit path; the input document is never changed. */
export function buildDirectExtrusion(document: DirectDocument, options: DirectExtrusionOptions): DirectBody | null {
  const sketches=options.sketchIds.map(id=>{const s=document.sketches.find(s=>s.id===id);if(!s)throw new DirectInputError('DIRECT_PROFILE_MISSING','Profile contour no longer exists.',undefined,[id]);return s})
  const tool=extrudeSketchProfile(sketches,options.height,options.offset)
  const target=document.bodies.find(b=>b.id===options.targetId)
  if(options.operation!=='new'&&!target)throw new DirectInputError('DIRECT_TARGET_BODY','Select a target body.')
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
/** All targets are computed before the document changes, so a refusal on any target leaves every body intact. */
export function applyDirectExtrusionProfile(document: DirectDocument, options: DirectExtrusionOptions): DirectDocument {
  const targets=directExtrusionTargets(options)
  if(options.operation==='new'){
    const body=buildDirectExtrusion(document,options)
    return parseDirectDocument(stringifyMeshJson({...document,bodies:[...document.bodies,...(body?[body]:[])]}))
  }
  const missing=targets.filter(id=>!document.bodies.some(b=>b.id===id))
  if(missing.length)throw new DirectInputError('DIRECT_TARGET_BODY','Select a target body.',undefined,missing)
  const results=new Map(targets.map(targetId=>[targetId,buildDirectExtrusion(document,{...options,targetId,targetIds:[]})]))
  const next={...document,bodies:document.bodies.flatMap(b=>{if(!results.has(b.id))return [b];const body=results.get(b.id);return body?[body]:[]})}
  return parseDirectDocument(stringifyMeshJson(next))
}
