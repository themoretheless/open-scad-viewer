import type {DirectBody} from './directModeling'
import type {CadOptions} from './cadWorkbench'
export interface CadJoint {parent:number;child:number;kind:string;axis:number[];origin:number[];position:number;min:number;max:number;parentShape:string;childShape:string}
// Elementwise compare is identical to JSON.stringify for finite-number vectors
// (old axes/origins are validated finite on read; -0===0 matches stringify "-0"→"0").
const sameVector=(a:number[],b:number[])=>a.length===b.length&&a.every((v,i)=>v===b[i])
const tag=/\/\* cad-joints-v1 ([A-Za-z0-9+/=]+) \*\//g
const fingerprintCache = new WeakMap<DirectBody, string>()
export function bodyFingerprint(body:DirectBody):string{
 const cached = fingerprintCache.get(body)
 if (cached !== undefined) return cached
 const {positions, indices} = body.mesh
 const ptCount = positions.length / 3
 const points = new Array<string>(ptCount)
 for (let i = 0; i < ptCount; i++) {
  const b = i * 3
  points[i] = `${Math.round(positions[b] * 1e5)},${Math.round(positions[b + 1] * 1e5)},${Math.round(positions[b + 2] * 1e5)}`
 }
 const triCount = indices.length / 3
 const triangles = new Array<string>(triCount)
 for (let i = 0; i < triCount; i++) {
  const b = i * 3
  let p0 = points[indices[b]], p1 = points[indices[b + 1]], p2 = points[indices[b + 2]]
  if (p0 > p1) { const t = p0; p0 = p1; p1 = t }
  if (p1 > p2) { const t = p1; p1 = p2; p2 = t }
  if (p0 > p1) { const t = p0; p0 = p1; p1 = t }
  triangles[i] = `${p0};${p1};${p2}`
 }
 triangles.sort()
 const joined = triangles.join('|')
 let h = 2166136261
 for (let i = 0; i < joined.length; i++) {
  h ^= joined.charCodeAt(i)
  h = Math.imul(h, 16777619)
 }
 const res = (h >>> 0).toString(16)
 fingerprintCache.set(body, res)
 return res
}
let lastJointSource: string | null = null
let lastJointJson = ''
export function readCadJoints(source:string):CadJoint[]{
 if (source === lastJointSource) return JSON.parse(lastJointJson)
 try{
  const match=[...source.matchAll(tag)].at(-1)
  if(!match)return []
  const value=JSON.parse(atob(match[1]))
  if(!Array.isArray(value)||value.length>100)return []
  const valid = value.filter(j=>Number.isInteger(j.parent)&&Number.isInteger(j.child)&&j.parent!==j.child&&['revolute','slider'].includes(j.kind)&&Array.isArray(j.axis)&&j.axis.length===3&&Array.isArray(j.origin)&&j.origin.length===3&&[...j.axis,...j.origin,j.min,j.max,j.position].every(Number.isFinite)&&Math.hypot(j.axis[0],j.axis[1],j.axis[2])>1e-12&&j.min<=j.position&&j.position<=j.max&&typeof j.parentShape==='string'&&typeof j.childShape==='string')
  lastJointSource = source
  lastJointJson = JSON.stringify(valid)
  return JSON.parse(lastJointJson)
 }catch{return []}
}
export function jointOptions(source:string,bodies:DirectBody[],o:CadOptions,min:number,max:number):CadOptions{
 if(!Number.isFinite(min)||!Number.isFinite(max)||min>max||o.amount<min||o.amount>max)throw Error('Joint position is outside its limits.')
 if(!Array.isArray(o.axis)||o.axis.length!==3||!o.axis.every(Number.isFinite)||Math.hypot(o.axis[0],o.axis[1],o.axis[2])<=1e-12)throw Error('Joint axis must be a nonzero finite vector.')
 if(o.ids.length!==2)throw Error('Choose parent then moving component.')
 const parent=Number(o.ids[0]),child=Number(o.ids[1]),old=readCadJoints(source).find(j=>j.parent===parent&&j.child===child)
 if(old){if(old.parentShape!==bodyFingerprint(bodies[parent])||old.childShape!==bodyFingerprint(bodies[child]))throw Error('Joint geometry changed. Remove the saved joint and create a new reference.');if(old.kind!==o.mode||!sameVector(old.axis,o.axis)||!sameVector(old.origin,o.origin))throw Error('Remove the saved joint before changing its axis, origin or kind.')}
 return {...o,amount:o.amount-(old?.position??0)}
}
export function saveCadJoint(source:string,bodies:DirectBody[],o:CadOptions,min:number,max:number):string{
 if(!Array.isArray(o.axis)||o.axis.length!==3||!o.axis.every(Number.isFinite)||Math.hypot(o.axis[0],o.axis[1],o.axis[2])<=1e-12)throw Error('Joint axis must be a nonzero finite vector.')
 const parent=Number(o.ids[0]),child=Number(o.ids[1]),j:CadJoint={parent,child,kind:o.mode,axis:[...o.axis],origin:[...o.origin],position:o.amount,min,max,parentShape:bodyFingerprint(bodies[parent]),childShape:bodyFingerprint(bodies[child])}
 const joints=readCadJoints(source).filter(old=>old.child!==child);joints.push(j);return source.replace(tag,'')+'\n/* cad-joints-v1 '+btoa(JSON.stringify(joints))+' */\n'
}
export const removeCadJoints=(source:string)=>source.replace(tag,'')
