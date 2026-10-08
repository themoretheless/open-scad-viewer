import {callGeometryRust} from './geometry/kernel'
import type {DirectSketch} from './directModeling'
import {withEditableSketchPath, SKETCH_PATH_TOLERANCE_MM} from './editableSketchPath'
import {withRetainedProfile} from './retainedSketchProfile'
import {authorBrepProfile} from './geometry/brepProfile'
import {outlineStrokeStyled, pathFromPolygon, pathFromCircle, pathToRing, simplifyPath, normalizePathRings} from './geometry/path2d'

function sourcePath(sketch:DirectSketch) {
  if(sketch.retainedProfile) throw Error('Select a source path, not an already filled region.')
  if(sketch.editablePath) return sketch.editablePath
  if(sketch.analytic?.kind==='circle') return pathFromCircle(sketch.analytic.center,sketch.analytic.radius)
  if(sketch.analytic) throw Error('Convert the arc to a source path before using this operation.')
  return pathFromPolygon(sketch.points,sketch.closed)
}

/** All stroke loops form one region: clockwise holes must not become separate solids. */
export function outlineSketchStroke(sketch:DirectSketch,width:number,id:string):DirectSketch {
  if(!Number.isFinite(width)||width<0.01||width>10000) throw Error('Stroke width must be between 0.01 and 10000 mm.')
  const paths=outlineStrokeStyled(sourcePath(sketch),width,{cap:'round',join:'round',tolerance:SKETCH_PATH_TOLERANCE_MM})
  const rings=paths.map(p=>pathToRing(p,SKETCH_PATH_TOLERANCE_MM))
  const profile=authorBrepProfile({kind:'polygon',rings})
  const source={...sketch,id,name:sketch.name+' · outline'}
  delete source.analytic;delete source.dimensions;delete source.editablePath
  return withRetainedProfile(source,profile)
}

/** Native simplification uses a user-selected tolerance; no topology certificate is implied. */
export function simplifySketchPath(sketch:DirectSketch,tolerance:number):DirectSketch {
  if(!Number.isFinite(tolerance)||tolerance<0.0001||tolerance>10) throw Error('Simplification tolerance must be between 0.0001 and 10 mm.')
  const path=simplifyPath(sourcePath(sketch),tolerance)
  const source={...sketch};delete source.analytic;delete source.dimensions
  return withEditableSketchPath(source,path)
}

/** Normalize all boundaries together so holes remain one filled region. */
export function cleanSketchContours(sketch:DirectSketch,tolerance:number,id:string):DirectSketch {
 if(!Number.isFinite(tolerance)||tolerance<0.0001||tolerance>10)throw Error('Cleanup tolerance must be between 0.0001 and 10 mm.')
 if(!sketch.closed){const cleaned=simplifySketchPath(sketch,tolerance);return {...cleaned,id,name:sketch.name+' · clean'}}
 if(sketch.retainedProfile){const profile=callGeometryRust<import('./geometry/brepProfile').BrepProfile>('brep_profile_cleanup',{loops:sketch.retainedProfile.loops,deviation:tolerance});return withRetainedProfile({...sketch,id,name:sketch.name+' · clean'},profile)}
 const input=[pathToRing(simplifyPath(sourcePath(sketch),tolerance),tolerance)]
 const rings=normalizePathRings(input,'evenodd')
 if(!rings.length)throw Error('Contour cleanup produced an empty region.')
 const source={...sketch,id,name:sketch.name+' · clean'}
 delete source.analytic;delete source.dimensions;delete source.editablePath;delete source.retainedProfile
 return withRetainedProfile(source,authorBrepProfile({kind:'polygon',rings}))
}
