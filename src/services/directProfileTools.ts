import {callGeometryRust} from './geometry/kernel'
import {requirePolylineSketch} from './retainedSketchProfile'
import { bakeSketch } from './directSketchGeometry'
import { normalizePolygonMesh, type PolygonMesh, booleanPolygonMeshes } from './geometry/polygon'
import { parseDirectDocument, type DirectSketch, type DirectBody, type DirectDocument, type Point2 } from './directModeling'
import { stringifyMeshJson } from './meshJson'

/** Bake a single corner to a sampled arc; never mutate the source sketch. */
export function directCornerTool(sketch: DirectSketch, vertex: number, radius: number, kind: 'fillet'|'dogear'): DirectSketch {
  requirePolylineSketch(sketch)
  if(!Number.isFinite(radius)) throw new Error('Radius must be at least 0.01 mm.')
  const points=callGeometryRust<Point2[]>('cad_sampled_corner',{sketch,vertex,radius,kind})
  const result={...bakeSketch(sketch),points};delete result.dimensions;return result
}

export interface DirectRevolveOptions { axis: 'x'|'y'; offset: number; angle: number; segments: number }
/** Rotate in the sketch's own XY plane around its horizontal or vertical axis. */
export function directRevolveTool(sketch: DirectSketch, options: DirectRevolveOptions): DirectBody {
  requirePolylineSketch(sketch)
  const mesh=normalizePolygonMesh(callGeometryRust<PolygonMesh>('cad_profile_revolve',{sketch,options}))
  const body={id:'preview-revolve',name:(sketch.name+' · revolve').slice(0,100),mesh:{positions:mesh.positions,indices:mesh.indices}}
  parseDirectDocument(stringifyMeshJson({version:1,sketches:[],bodies:[body]}))
  return body
}

export function applyDirectRevolve(document: DirectDocument, sketchId:string, options:DirectRevolveOptions, operation:'new'|'union'|'difference', targetId:string, id:string):DirectDocument {
  const next=parseDirectDocument(stringifyMeshJson(document)),sketch=next.sketches.find(s=>s.id===sketchId)
  if(!sketch) throw new Error('Select a sketch.')
  const tool=directRevolveTool(sketch,options)
  if(operation==='new') next.bodies.push({...tool,id})
  else {
    const target=next.bodies.find(b=>b.id===targetId)
    if(!target) throw new Error('Select the target body.')
    const mesh=booleanPolygonMeshes(target.mesh,tool.mesh,operation)
    if(!mesh.indices.length) next.bodies=next.bodies.filter(b=>b.id!==targetId)
    else target.mesh={positions:mesh.positions,indices:mesh.indices}
  }
  return parseDirectDocument(stringifyMeshJson(next))
}
