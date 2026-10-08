import type {DirectSketch,Point2} from './directModeling'
import {authorBrepProfile,booleanBrepProfiles,offsetBrepProfile,transformBrepProfile,validateBrepProfile,type BrepProfile,type BrepProfileBooleanOperation} from './geometry/brepProfile'
import {evaluateNurbsCurve} from './nurbsCurve'
import {callGeometryRust} from './geometry/kernel'

/** Display samples only. All modelling uses retainedProfile.loops. */
export function retainedProfileDisplay(profile:BrepProfile):Point2[][] {
 return profile.loops.map(loop=>loop.flatMap(curve=>{
  const count=curve.degree===1?1:24,a=curve.knots[curve.degree],b=curve.knots[curve.controlPoints.length]
  return Array.from({length:count},(_,i)=>evaluateNurbsCurve(curve,a+(b-a)*i/count).point as Point2)
 }))
}
export function sketchProfile(sketch:DirectSketch):BrepProfile {
 if(sketch.retainedProfile)return validateBrepProfile(sketch.retainedProfile.loops,'material-left',sketch.retainedProfile.toleranceMm)
 if(!sketch.closed)throw Error('Exact profile requires closed contours.')
 if(sketch.editablePath)return authorBrepProfile({kind:'bezier',path:sketch.editablePath})
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
 if(sketches.some(s=>{const p=s.plane??{origin:[0,0,0],u:[1,0,0],v:[0,1,0]};return ![p.origin,p.u,p.v].every((a,i)=>a.every((x,j)=>x===[plane.origin,plane.u,plane.v][i][j]))}))throw Error('Exact profile operations require a common sketch plane.')
 const profile=sketches.slice(1).reduce((a,s)=>booleanBrepProfiles(a,sketchProfile(s),operation),sketchProfile(sketches[0]))
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
