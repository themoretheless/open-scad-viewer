import type {DirectSketch,Point2} from './directModeling'
import {authorBrepProfile,booleanBrepProfiles,offsetBrepProfile,signedAreaBrepProfileLoop,transformBrepProfile,validateBrepProfile,type BrepProfile,type BrepProfileBooleanOperation} from '../kernel/geometry/brepProfile'
import {callGeometryRust} from '../kernel/geometry/kernel'

const profileDisplayCache = new WeakMap<BrepProfile, Point2[][]>()
/** Display samples only. All modelling uses retainedProfile.loops. */
export function retainedProfileDisplay(profile:BrepProfile):Point2[][] {
 const cached = profileDisplayCache.get(profile)
 if (cached) return cached
 const display = callGeometryRust<Point2[][]>('brep_profile_display',{loops:profile.loops})
 profileDisplayCache.set(profile, display)
 return display
}
export function sketchProfile(sketch:DirectSketch):BrepProfile {
 if(sketch.retainedProfile)return validateBrepProfile(sketch.retainedProfile.loops,'material-left',sketch.retainedProfile.toleranceMm)
 if(!sketch.closed)throw Error('Exact profile requires closed contours.')
 if(sketch.analytic?.kind==='circle'){
  const {center,radius}=sketch.analytic
  return transformBrepProfile(authorBrepProfile({kind:'circle',radius}),[1,0,0,0,0,1,0,0,0,0,1,0,center[0],center[1],0,1])
 }
 if(sketch.analytic)throw Error('Open arcs require endpoint assembly before creating a profile.')
 return authorBrepProfile({kind:'polygon',rings:[sketch.points]})
}
export function withRetainedProfile(sketch:DirectSketch,profile:BrepProfile):DirectSketch {
 if(!profile.loops.length)throw Error('The profile contains no material.')
 const next={...sketch,retainedProfile:profile,points:retainedProfileDisplay(profile)[0],closed:true}
 delete next.analytic;delete next.dimensions;delete next.constraints;delete next.editablePath
 return next
}
export function combineSketchProfiles(sketches:DirectSketch[],operation:BrepProfileBooleanOperation):DirectSketch {
 if(!['union','difference','intersection'].includes(operation))throw Error('Unsupported sketch profile operation.')
 if(sketches.length<2)throw Error('Select at least two closed profiles.')
 const plane=sketches[0].plane??{origin:[0,0,0],u:[1,0,0],v:[0,1,0]}
 const po=plane.origin,pu=plane.u,pv=plane.v
 if(sketches.some(s=>{const p=s.plane??{origin:[0,0,0],u:[1,0,0],v:[0,1,0]},so=p.origin,su=p.u,sv=p.v;return so[0]!==po[0]||so[1]!==po[1]||so[2]!==po[2]||su[0]!==pu[0]||su[1]!==pu[1]||su[2]!==pu[2]||sv[0]!==pv[0]||sv[1]!==pv[1]||sv[2]!==pv[2]}))throw Error('Exact profile operations require a common sketch plane.')
 let profile=sketchProfile(sketches[0])
 for(let i=1;i<sketches.length;i++)profile=booleanBrepProfiles(profile,sketchProfile(sketches[i]),operation)
 return withRetainedProfile(sketches[0],profile)
}
export interface SketchProfileRegion {loops:number[];areaMm2:number}
const insidePolygon=(point:Point2,polygon:Point2[])=>{let inside=false;for(let i=0,j=polygon.length-1;i<polygon.length;j=i++){const [xi,yi]=polygon[i],[xj,yj]=polygon[j];if((yi>point[1])!==(yj>point[1])&&point[0]<(xj-xi)*(point[1]-yi)/(yj-yi)+xi)inside=!inside}return inside}
/** Connected material regions of a retained profile: each outer loop with the holes it directly owns, in outer-loop order. */
export function sketchProfileRegions(profile:BrepProfile):SketchProfileRegion[] {
 const areas=profile.loops.map(loop=>signedAreaBrepProfileLoop(loop,profile.toleranceMm)),display=retainedProfileDisplay(profile)
 const regions=areas.flatMap((area,i)=>area>0?[{loops:[i],areaMm2:area}]:[])
 areas.forEach((area,i)=>{
  if(area>0)return
  const probe=display[i]?.[0],owners=probe?regions.filter(r=>insidePolygon(probe,display[r.loops[0]]??[])):[]
  const owner=owners.sort((a,b)=>areas[a.loops[0]]-areas[b.loops[0]])[0]
  if(!owner)throw Error('A profile hole has no enclosing region.')
  owner.loops.push(i);owner.areaMm2+=area
 })
 return regions
}
/** Keep only the listed region indexes of a retained profile sketch. */
export function selectSketchProfileRegions(sketch:DirectSketch,excluded:readonly number[]):DirectSketch {
 if(!excluded.length)return sketch
 const profile=sketchProfile(sketch),regions=sketchProfileRegions(profile)
 if(excluded.some(i=>!Number.isInteger(i)||i<0||i>=regions.length))throw Error('A selected profile region no longer exists.')
 const kept=regions.filter((_,i)=>!excluded.includes(i))
 if(!kept.length)throw Error('Keep at least one profile region.')
 return withRetainedProfile(sketch,validateBrepProfile(kept.flatMap(r=>r.loops).sort((a,b)=>a-b).map(i=>profile.loops[i]),'material-left',profile.toleranceMm))
}
export const unionSketchProfiles=(sketches:DirectSketch[]):DirectSketch=>combineSketchProfiles(sketches,'union')
export function transformRetainedSketch(sketch:DirectSketch,delta:Point2,angle:number,scale:number,pivot?:Point2):DirectSketch {
 const profile=structuredClone(sketch.retainedProfile!),curves=profile.loops.flat(),points=curves.flatMap(c=>c.controlPoints)
 const transformed=callGeometryRust<{points:number[][]}>('cad_transform_sketch',{sketch:{points},delta,angle,scale,pivot:pivot??null})
 let offset=0;for(const curve of curves){curve.controlPoints=transformed.points.slice(offset,offset+curve.controlPoints.length);offset+=curve.controlPoints.length}
 return withRetainedProfile(sketch,validateBrepProfile(profile.loops,'material-left',profile.toleranceMm))
}
export function requirePolylineSketch(sketch:DirectSketch){if(sketch.editablePath)throw Error('Edit the source Bézier path before using this polygon-only operation.');if(sketch.retainedProfile)throw Error('This operation does not yet support retained curve profiles. Edit the source curves or use a supported profile operation.')}

export const offsetRetainedSketch=(sketch:DirectSketch,distance:number):DirectSketch=>withRetainedProfile(sketch,offsetBrepProfile(sketchProfile(sketch),distance))
