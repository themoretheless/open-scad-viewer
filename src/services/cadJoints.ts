import type {DirectBody} from './directModeling'
import type {CadOptions} from './cadWorkbench'
export interface CadJoint {parent:number;child:number;kind:string;axis:number[];origin:number[];position:number;min:number;max:number;parentShape:string;childShape:string}
// Elementwise compare is identical to JSON.stringify for finite-number vectors
// (old axes/origins are validated finite on read; -0===0 matches stringify "-0"→"0").
const sameVector=(a:number[],b:number[])=>a.length===b.length&&a.every((v,i)=>v===b[i])
const tag=/\/\* cad-joints-v1 ([A-Za-z0-9+/=]+) \*\//g
export function bodyFingerprint(body:DirectBody):string{const points=Array.from({length:body.mesh.positions.length/3},(_,i)=>body.mesh.positions.slice(i*3,i*3+3).map(v=>Math.round(v*1e5)).join(',')),triangles=Array.from({length:body.mesh.indices.length/3},(_,i)=>Array.from(body.mesh.indices.slice(i*3,i*3+3),j=>points[j]).sort().join(';')).sort();let h=2166136261;for(const c of triangles.join('|')){h^=c.charCodeAt(0);h=Math.imul(h,16777619)}return (h>>>0).toString(16)}
export function readCadJoints(source:string):CadJoint[]{try{const match=[...source.matchAll(tag)].at(-1);if(!match)return [];const value=JSON.parse(atob(match[1]));if(!Array.isArray(value)||value.length>100)return [];return value.filter(j=>Number.isInteger(j.parent)&&Number.isInteger(j.child)&&j.parent!==j.child&&['revolute','slider'].includes(j.kind)&&Array.isArray(j.axis)&&j.axis.length===3&&Array.isArray(j.origin)&&j.origin.length===3&&[...j.axis,...j.origin,j.min,j.max,j.position].every(Number.isFinite)&&j.min<=j.position&&j.position<=j.max&&typeof j.parentShape==='string'&&typeof j.childShape==='string')}catch{return []}}
export function jointOptions(source:string,bodies:DirectBody[],o:CadOptions,min:number,max:number):CadOptions{
 if(!Number.isFinite(min)||!Number.isFinite(max)||min>max||o.amount<min||o.amount>max)throw Error('Joint position is outside its limits.')
 if(o.ids.length!==2)throw Error('Choose parent then moving component.')
 const parent=Number(o.ids[0]),child=Number(o.ids[1]),old=readCadJoints(source).find(j=>j.parent===parent&&j.child===child)
 if(old){if(old.parentShape!==bodyFingerprint(bodies[parent])||old.childShape!==bodyFingerprint(bodies[child]))throw Error('Joint geometry changed. Remove the saved joint and create a new reference.');if(old.kind!==o.mode||!sameVector(old.axis,o.axis)||!sameVector(old.origin,o.origin))throw Error('Remove the saved joint before changing its axis, origin or kind.')}
 return {...o,amount:o.amount-(old?.position??0)}
}
export function saveCadJoint(source:string,bodies:DirectBody[],o:CadOptions,min:number,max:number):string{
 const parent=Number(o.ids[0]),child=Number(o.ids[1]),j:CadJoint={parent,child,kind:o.mode,axis:[...o.axis],origin:[...o.origin],position:o.amount,min,max,parentShape:bodyFingerprint(bodies[parent]),childShape:bodyFingerprint(bodies[child])}
 const joints=readCadJoints(source).filter(old=>old.child!==child);joints.push(j);return source.replace(tag,'')+'\n/* cad-joints-v1 '+btoa(JSON.stringify(joints))+' */\n'
}
export const removeCadJoints=(source:string)=>source.replace(tag,'')
