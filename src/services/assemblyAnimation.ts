import {parseDirectDocument,serializeDirectDocument,type DirectDocument,type DirectBody} from './directModeling'
import {applySolidSceneEdit} from './solidSceneEdit'
export type AssemblyEasing = 'linear' | 'easeInOutCubic' | 'easeOutBack'
export interface AssemblyPose {id:string;translation:[number,number,number];angle:number;rotation?:[number,number,number];scale:number}
export interface AssemblyKeyframe {time:number;poses:AssemblyPose[];camera:{yaw:number;pitch:number};easing?:AssemblyEasing}
export interface AssemblyAnimation {version:1;frames:AssemblyKeyframe[];easing?:AssemblyEasing}
export interface PoseBoundingBoxOverlap {
 aId:string
 bId:string
 overlapVolumeMm3:number
 bounds:{min:[number,number,number];max:[number,number,number]}
}
const TAU = Math.PI * 2
function remEuclidTau(x: number): number {
 const r = x % TAU
 return r < 0 ? (r + TAU === TAU ? 0 : r + TAU) : r
}
export function evaluateAssemblyEasing(t: number, easing: AssemblyEasing = 'linear'): number {
 const clamped = Math.max(0, Math.min(1, t))
 if (easing === 'easeInOutCubic') {
  return clamped < 0.5
   ? 4 * clamped * clamped * clamped
   : 1 - Math.pow(-2 * clamped + 2, 3) / 2
 }
 if (easing === 'easeOutBack') {
  const c1 = 1.70158
  const c3 = c1 + 1
  return 1 + c3 * Math.pow(clamped - 1, 3) + c1 * Math.pow(clamped - 1, 2)
 }
 return clamped
}
function sampleKeyframes(times: readonly number[], values: readonly number[][], time: number, angles: readonly number[], angleSet?: ReadonlySet<number>, easings?: readonly (AssemblyEasing | undefined)[]): number[] {
 if (!times.length || times.length > 64 || times.length !== values.length || !Number.isFinite(time)) {
  throw new Error('Invalid keyframe timeline')
 }
 const width = values[0].length
 if (
  width === 0 ||
  width > 322 ||
  values.some(v => v.length !== width || v.some(x => !Number.isFinite(x) || Math.abs(x) > 1e6)) ||
  times.some((x, i) => !Number.isFinite(x) || x < 0 || x > 120 || (i > 0 && times[i - 1] >= x)) ||
  angles.some(i => i < 0 || i >= width)
 ) {
  throw new Error('Invalid keyframe values')
 }
 if (time <= times[0]) return values[0].slice()
 if (time >= times[times.length - 1]) return values[values.length - 1].slice()
 let lo = 0, hi = times.length - 1
 while (lo < hi) {
  const mid = (lo + hi) >>> 1
  if (times[mid] < time) lo = mid + 1
  else hi = mid
 }
 const upper = lo
 const rawT = (time - times[upper - 1]) / (times[upper] - times[upper - 1])
 const t = evaluateAssemblyEasing(rawT, easings?.[upper - 1] ?? 'linear')
 const aRow = values[upper - 1]
 const bRow = values[upper]
 const set = angleSet ?? new Set(angles)
 const out = new Array<number>(width)
 for (let i = 0; i < width; i++) {
  const a = aRow[i]
  const b = bRow[i]
  const delta = set.has(i) ? remEuclidTau(b - a + Math.PI) - Math.PI : b - a
  out[i] = a + delta * t
 }
 return out
}
const VALID_EASINGS = new Set<AssemblyEasing>(['linear', 'easeInOutCubic', 'easeOutBack'])
export function validateAssemblyAnimation(document:DirectDocument,animation:AssemblyAnimation):void {
 if(!animation||animation.version!==1||!Array.isArray(animation.frames)||!animation.frames.length||animation.frames.length>64)throw Error('Expected 1–64 animation keyframes.')
 if(animation.easing!==undefined&&!VALID_EASINGS.has(animation.easing))throw Error('Invalid animation easing.')
 const ids=animation.frames[0].poses.map(p=>p.id)
 const validBodyIds=new Set<string>()
 for(const b of document.bodies)if(!b.instance)validBodyIds.add(b.id)
 if(ids.length>16||new Set(ids).size!==ids.length||ids.some(id=>!validBodyIds.has(id)))throw Error('Animation requires up to 16 independent bodies.')
 let previous=-1
 for(const frame of animation.frames){
  if(!Number.isFinite(frame.time)||frame.time<0||frame.time>120||frame.time<=previous)throw Error('Frame times must increase within 0–120 seconds.')
  if(frame.easing!==undefined&&!VALID_EASINGS.has(frame.easing))throw Error('Invalid frame easing.')
  previous=frame.time
  if(!frame.camera||![frame.camera.yaw,frame.camera.pitch].every(v=>Number.isFinite(v)&&Math.abs(v)<=Math.PI*100)||Math.abs(frame.camera.pitch)>Math.PI/2)throw Error('Invalid frame camera.')
  if(frame.poses.length!==ids.length||frame.poses.some((p,i)=>p.id!==ids[i]||!Array.isArray(p.translation)||p.translation.length!==3||p.translation.some(v=>!Number.isFinite(v)||Math.abs(v)>100000)||!Number.isFinite(p.angle)||Math.abs(p.angle)>36000||(p.rotation!==undefined&&(!Array.isArray(p.rotation)||p.rotation.length!==3||p.rotation.some(v=>!Number.isFinite(v)||Math.abs(v)>36000)))||!Number.isFinite(p.scale)||p.scale<.001||p.scale>100))throw Error('All frames must use the same ordered body tracks and valid poses.')
 }
}
const DEG_TO_RAD = Math.PI / 180
const RAD_TO_DEG = 180 / Math.PI
export function sampleAssemblyAnimation(document:DirectDocument,animation:AssemblyAnimation,time:number,easing?:AssemblyEasing):{document:DirectDocument;camera:{yaw:number;pitch:number}} {
 validateAssemblyAnimation(document,animation)
 const firstPoses=animation.frames[0].poses,count=firstPoses.length
 const ids=new Array<string>(count)
 for(let i=0;i<count;i++)ids[i]=firstPoses[i].id
 const width=count*7+2,frameCount=animation.frames.length
 const times=new Array<number>(frameCount)
 const values=new Array<number[]>(frameCount)
 const easings=new Array<AssemblyEasing|undefined>(frameCount)
 for(let fIdx=0;fIdx<frameCount;fIdx++){
  const f=animation.frames[fIdx]
  times[fIdx]=f.time
  easings[fIdx]=easing??f.easing??animation.easing??'linear'
  const row=new Array<number>(width)
  for(let i=0;i<count;i++){
   const p=f.poses[i],base=i*7,rot=p.rotation
   row[base]=p.translation[0]
   row[base+1]=p.translation[1]
   row[base+2]=p.translation[2]
   row[base+3]=rot?rot[0]*DEG_TO_RAD:0
   row[base+4]=rot?rot[1]*DEG_TO_RAD:0
   row[base+5]=(rot?rot[2]:p.angle)*DEG_TO_RAD
   row[base+6]=p.scale
  }
  row[count*7]=f.camera.yaw
  row[count*7+1]=f.camera.pitch
  values[fIdx]=row
 }
 const angles=new Array<number>(count*3+1)
 for(let i=0;i<count;i++){angles[i*3]=i*7+3;angles[i*3+1]=i*7+4;angles[i*3+2]=i*7+5}
 angles[count*3]=count*7
 const sampled=sampleKeyframes(times,values,time,angles,new Set(angles),easings)
 let next=parseDirectDocument(serializeDirectDocument(document))
 for(let i=0;i<count;i++){
  const id=ids[i],base=i*7
  for(const [axis,k] of [['x',3],['y',4],['z',5]] as const){
   if(sampled[base+k]===0&&axis!=='z')continue
   next=applySolidSceneEdit(next,{operation:'transform',id,ids:[id],createdId:'',x:axis==='z'?sampled[base]:0,y:axis==='z'?sampled[base+1]:0,z:axis==='z'?sampled[base+2]:0,axis,angle:sampled[base+k]*RAD_TO_DEG,scale:axis==='z'?sampled[base+6]:1})
  }
 }
 return {document:next,camera:{yaw:sampled[count*7],pitch:sampled[count*7+1]}}
}
function bodyAabb(body:DirectBody):{min:[number,number,number];max:[number,number,number]}|null {
 const pos=body.mesh.positions
 if(!pos||pos.length<3)return null
 let minX=Infinity,minY=Infinity,minZ=Infinity,maxX=-Infinity,maxY=-Infinity,maxZ=-Infinity
 for(let i=0;i<pos.length;i+=3){
  const x=pos[i],y=pos[i+1],z=pos[i+2]
  if(x<minX)minX=x;if(x>maxX)maxX=x
  if(y<minY)minY=y;if(y>maxY)maxY=y
  if(z<minZ)minZ=z;if(z>maxZ)maxZ=z
 }
 return {min:[minX,minY,minZ],max:[maxX,maxY,maxZ]}
}
export function findPoseBoundingBoxOverlaps(documentOrBodies:DirectDocument|readonly DirectBody[],epsilon=1e-6):PoseBoundingBoxOverlap[] {
 const bodies=Array.isArray(documentOrBodies)?documentOrBodies:(documentOrBodies as DirectDocument).bodies
 const boxes=bodies
  .map((b,order)=>({id:b.id,order,box:bodyAabb(b)}))
  .filter((item):item is {id:string;order:number;box:NonNullable<ReturnType<typeof bodyAabb>>}=>item.box!==null)
  .sort((a,b)=>a.box.min[0]-b.box.min[0]||a.order-b.order)
 const overlaps:PoseBoundingBoxOverlap[]=[]
 for(let i=0;i<boxes.length;i++){
  const a=boxes[i]
  const maxAX=a.box.max[0]-epsilon
  for(let j=i+1;j<boxes.length;j++){
   const b=boxes[j]
   if(b.box.min[0]>=maxAX)break
   const minX=Math.max(a.box.min[0],b.box.min[0]),maxX=Math.min(a.box.max[0],b.box.max[0])
   const minY=Math.max(a.box.min[1],b.box.min[1]),maxY=Math.min(a.box.max[1],b.box.max[1])
   const minZ=Math.max(a.box.min[2],b.box.min[2]),maxZ=Math.min(a.box.max[2],b.box.max[2])
   const dx=maxX-minX,dy=maxY-minY,dz=maxZ-minZ
   if(dx>epsilon&&dy>epsilon&&dz>epsilon){
    const first=a.order<b.order?a:b
    const second=a.order<b.order?b:a
    overlaps.push({
     aId:first.id,
     bId:second.id,
     overlapVolumeMm3:dx*dy*dz,
     bounds:{min:[minX,minY,minZ],max:[maxX,maxY,maxZ]},
    })
   }
  }
 }
 return overlaps
}
