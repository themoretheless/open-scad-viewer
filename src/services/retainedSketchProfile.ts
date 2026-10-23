import type {DirectSketch,Point2} from './directModeling'
import {authorBrepProfile,booleanBrepProfiles,offsetBrepProfile,transformBrepProfile,validateBrepProfile,type BrepProfile,type BrepProfileBooleanOperation} from './geometry/brepProfile'
import {callGeometryRust} from './geometry/kernel'

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
 delete next.analytic;delete next.dimensions;delete next.editablePath
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
export const unionSketchProfiles=(sketches:DirectSketch[]):DirectSketch=>combineSketchProfiles(sketches,'union')
export function transformRetainedSketch(sketch:DirectSketch,delta:Point2,angle:number,scale:number,pivot?:Point2):DirectSketch {
 const profile=structuredClone(sketch.retainedProfile!),curves=profile.loops.flat(),points=curves.flatMap(c=>c.controlPoints)
 const transformed=callGeometryRust<{points:number[][]}>('cad_transform_sketch',{sketch:{points},delta,angle,scale,pivot:pivot??null})
 let offset=0;for(const curve of curves){curve.controlPoints=transformed.points.slice(offset,offset+curve.controlPoints.length);offset+=curve.controlPoints.length}
 return withRetainedProfile(sketch,validateBrepProfile(profile.loops,'material-left',profile.toleranceMm))
}
export function requirePolylineSketch(sketch:DirectSketch){if(sketch.editablePath)throw Error('Edit the source Bézier path before using this polygon-only operation.');if(sketch.retainedProfile)throw Error('This operation does not yet support retained curve profiles. Edit the source curves or use a supported profile operation.')}

export const offsetRetainedSketch=(sketch:DirectSketch,distance:number):DirectSketch=>withRetainedProfile(sketch,offsetBrepProfile(sketchProfile(sketch),distance))
