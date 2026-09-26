import { bakeSketch, worldPoint } from './directSketchGeometry'
import { revolvePolygonProfile, booleanPolygonMeshes } from './geometry/polygon'
import { parseDirectDocument, type DirectSketch, type DirectBody, type DirectDocument, type Point2 } from './directModeling'
import { stringifyMeshJson } from './meshJson'

const cross = (a: Point2, b: Point2, c: Point2) => (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
function validateContour(points: Point2[]) {
  if (points.length > 512 || points.length < 3 || !points.every(p=>p.every(v=>Number.isFinite(v)&&Math.abs(v)<=1e6))) throw new Error('Invalid contour or point budget exceeded.')
  const n=points.length
  for(let i=0;i<n;i++) {
    const a=points[i],b=points[(i+1)%n]
    if(Math.hypot(a[0]-b[0],a[1]-b[1])<1e-8) throw new Error('Contour has a zero-length edge.')
    for(let j=i+2;j<n;j++) {
      if(i===0&&j===n-1) continue
      const c=points[j],d=points[(j+1)%n]
      if(Math.max(a[0],b[0])+1e-9<Math.min(c[0],d[0])||Math.max(c[0],d[0])+1e-9<Math.min(a[0],b[0])||Math.max(a[1],b[1])+1e-9<Math.min(c[1],d[1])||Math.max(c[1],d[1])+1e-9<Math.min(a[1],b[1])) continue
      if(cross(a,b,c)*cross(a,b,d)<=0&&cross(c,d,a)*cross(c,d,b)<=0) throw new Error('Radius creates a self-intersection. Use a smaller radius.')
    }
  }
}
/** Bake a single corner to a sampled arc; never mutate the source sketch. */
export function directCornerTool(sketch: DirectSketch, vertex: number, radius: number, kind: 'fillet'|'dogear'): DirectSketch {
  if(!sketch.closed || !Number.isInteger(vertex) || vertex<0 || vertex>=sketch.points.length) throw new Error('Select a corner of a closed contour.')
  if(!Number.isFinite(radius)||radius<.01||radius>1e6) throw new Error('Radius must be at least 0.01 mm.')
  validateContour(sketch.points)
  const p=sketch.points, n=p.length, v=p[vertex], prev=p[(vertex+n-1)%n], next=p[(vertex+1)%n]
  const la=Math.hypot(prev[0]-v[0],prev[1]-v[1]),lb=Math.hypot(next[0]-v[0],next[1]-v[1])
  const u:Point2=[(prev[0]-v[0])/la,(prev[1]-v[1])/la],w:Point2=[(next[0]-v[0])/lb,(next[1]-v[1])/lb]
  const theta=Math.acos(Math.max(-1,Math.min(1,u[0]*w[0]+u[1]*w[1])))
  if(theta<.01||Math.PI-theta<.01) throw new Error('Select a non-collinear corner.')
  if(kind==='dogear'&&Math.abs(theta-Math.PI/2)>.001) throw new Error('DogEar requires a right-angle corner.')
  const distance=kind==='fillet'?radius/Math.tan(theta/2):Math.SQRT2*radius
  if(distance>=Math.min(la,lb)-1e-8) throw new Error('Radius exceeds the adjacent edges. Use a smaller radius.')
  const a:Point2=[v[0]+u[0]*distance,v[1]+u[1]*distance],b:Point2=[v[0]+w[0]*distance,v[1]+w[1]*distance]
  const center:Point2=kind==='dogear'?[(a[0]+b[0])/2,(a[1]+b[1])/2]:[v[0]+(u[0]+w[0])*distance/(1+Math.cos(theta)),v[1]+(u[1]+w[1])*distance/(1+Math.cos(theta))]
  const start=Math.atan2(a[1]-center[1],a[0]-center[0]),turn=Math.sign(cross(prev,v,next))
  const sweep=kind==='fillet'?turn*(Math.PI-theta):turn*Math.PI
  const segments=Math.ceil(Math.abs(sweep)/(Math.PI/(kind==='dogear'?72:36)))
  const arc=Array.from({length:segments+1},(_,i):Point2=>i===0?a:i===segments?b:[center[0]+radius*Math.cos(start+sweep*i/segments),center[1]+radius*Math.sin(start+sweep*i/segments)])
  const points=[...p.slice(0,vertex),...arc,...p.slice(vertex+1)].map(p=>[...p] as Point2)
  validateContour(points)
  return {...bakeSketch(sketch),points}
}

export interface DirectRevolveOptions { axis: 'x'|'y'; offset: number; angle: number; segments: number }
/** Rotate in the sketch's own XY plane around its horizontal or vertical axis. */
export function directRevolveTool(sketch: DirectSketch, options: DirectRevolveOptions): DirectBody {
  const {axis,offset,angle,segments}=options
  if(!sketch.closed) throw new Error('Close the contour before revolving.')
  validateContour(sketch.points)
  if(!['x','y'].includes(axis)||!Number.isFinite(offset)||Math.abs(offset)>1e6||!Number.isFinite(angle)||Math.abs(angle)<.1||Math.abs(angle)>360||!Number.isInteger(segments)||segments<8||segments>128) throw new Error('Use a nonzero angle up to 360° and 8–128 segments.')
  const radial=sketch.points.map(p=>(axis==='y'?p[0]:p[1])-offset)
  if(Math.min(...radial)<-1e-8&&Math.max(...radial)>1e-8) throw new Error('The profile crosses the rotation axis. Move the axis outside the contour.')
  const side=Math.max(...radial)>1e-8?1:-1
  const profile=sketch.points.map((p,i)=>[Math.max(0,side*radial[i]),axis==='y'?p[1]:p[0]])
  const mesh=revolvePolygonProfile(profile,angle,segments,true)
  // Both mappings preserve orientation. At angle 0 the profile coincides with its 2D sketch.
  const positions:number[]=[]
  for(let i=0;i<mesh.positions.length;i+=3) {
    const [r,t,h]=mesh.positions.slice(i,i+3)
    positions.push(...worldPoint(axis==='y'?[offset+side*r,h,-side*t]:[h,offset+side*r,side*t],sketch.plane))
  }
  const body={id:'preview-revolve',name:(sketch.name+' · revolve').slice(0,100),mesh:{positions:Float64Array.from(positions),indices:mesh.indices.slice()}}
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
