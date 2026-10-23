import type {DirectBody,DirectDocument} from './directModeling'
import {booleanNurbsBrep,tessellateNurbsBrep,type BrepBooleanOperation} from './geometry/brep'
import {booleanPolygonMeshes} from './geometry/polygon'
import {mergeByDistance} from './meshEditing'

export interface SolidBooleanOptions {
 operation:BrepBooleanOperation
 a:string[]
 b:string[]
 segments:number
 ru:boolean
}
export interface SolidBooleanResult {
 document:DirectDocument
 resultId:string|null
 exact:boolean
 toleranceMm:number
}

/** Compute a transaction on an isolated document; the UI owns commit and history. */
export function applySolidBoolean(source:DirectDocument,options:SolidBooleanOptions):SolidBooleanResult {
 const d=structuredClone(source),{operation,a,b,segments,ru}=options
 const label=(r:string,e:string)=>ru?r:e
 if(!a.length||!b.length)throw Error(label('Заполните оба поля: A — из чего вычитаем, B — что вычитаем.','Fill both fields: A is what to subtract from, B is what to subtract.'))
 const ids=[...a,...b]
 if(new Set(ids).size!==ids.length)throw Error(label('Тело должно быть только в одном поле A или B.','Each body must occur once, in either A or B.'))
 const bodyOf=(id:string)=>{const body=d.bodies.find(item=>item.id===id);if(!body)throw Error(label('Тело больше не существует.','A body no longer exists.'));return body}
 let exact=true,toleranceMm=0
 const combine=(left:DirectBody,right:DirectBody,op:BrepBooleanOperation)=>{
  const r=booleanPair(left,right,op,segments,label)
  exact&&=r.exact;toleranceMm=Math.max(toleranceMm,r.toleranceMm??0)
  return r.body
 }
 const fold=(values:string[])=>values.slice(1).reduce((acc,id)=>{
  const result=combine(acc,bodyOf(id),'union')
  if(!result)throw Error(label('Результат пуст: тела не пересекаются так, как требует операция.','The result is empty: the bodies do not overlap the way this operation needs.'))
  return result
 },bodyOf(values[0]))
 const result=combine(fold(a),fold(b),operation),removed=new Set(ids)
 d.bodies=d.bodies.flatMap(body=>body.id===a[0]?(result?[result]:[]):removed.has(body.id)?[]:[body])
 return {document:d,resultId:result?.id??null,exact,toleranceMm}
}

/**
 * One boolean between two bodies.
 *
 * Two exact bodies are combined exactly or not at all: the Solid workspace promises B-rep
 * results, so a kernel refusal is reported with the kernel's reason instead of being
 * papered over with a polygon boolean that would quietly demote the result to a mesh.
 * The welded mesh boolean remains only for bodies that never had exact topology.
 */
/** Exact results keep the kernel tolerance (1e-7 mm); a tolerant fallback reports its own, above 1e-6 mm. */
const TOLERANT_ABOVE_MM=1e-6
function booleanPair(a:DirectBody,b:DirectBody,operation:BrepBooleanOperation,segments:number,label:(ru:string,en:string)=>string):{body:DirectBody|null;exact:boolean;toleranceMm?:number}{
 if(a.brep&&b.brep){
  try{
   const brep=booleanNurbsBrep(a.brep,b.brep,operation)
   const built=tessellateNurbsBrep(brep,segments)
   const result={...a,brep,mesh:{positions:built.positions.slice(),indices:built.indices.slice()}}
   const toleranceMm=brep.toleranceMm>TOLERANT_ABOVE_MM?brep.toleranceMm:undefined
   return {body:result.brep.bodies.length?result:null,exact:true,toleranceMm}
  }catch(refusal){
   // The kernel certifies each surface pairing it walks; an uncertified pairing is a
   // capability limit of the exact kernel, not a defect in the model, and the bodies are
   // left untouched rather than rebuilt from polygons.
   const reason=refusal instanceof Error?refusal.message:String(refusal)
   throw Error(label(`Ядро не может выполнить точную операцию для этой пары тел (${a.name} и ${b.name}), тела не изменены. Причина ядра: ${reason}`,`The kernel cannot perform the exact operation for this pair of bodies (${a.name} and ${b.name}); the bodies are unchanged. Kernel reason: ${reason}`))
  }
 }
 if(operation==='xor')throw Error(label('XOR доступен только для двух точных тел B-rep.','XOR is available only for two exact B-rep bodies.'))
 // Tessellated meshes can carry duplicated seam vertices, which the BSP boolean rejects as
 // unstitched. Weld each input at a size-relative tolerance, then retry once coarser.
 const extent=(mesh:{positions:ArrayLike<number>})=>{let span=0;for(let axis=0;axis<3;axis++){let min=Infinity,max=-Infinity;for(let i=axis;i<mesh.positions.length;i+=3){min=Math.min(min,mesh.positions[i]);max=Math.max(max,mesh.positions[i])}span=Math.max(span,max-min)}return span}
 const scale=Math.max(extent(a.mesh),extent(b.mesh))
 const weldedBoolean=(tolerance:number)=>booleanPolygonMeshes(mergeByDistance(a.mesh,tolerance),mergeByDistance(b.mesh,tolerance),operation as 'union'|'difference'|'intersection')
 let built
 try{built=weldedBoolean(scale*1e-6)}catch{built=weldedBoolean(scale*1e-4)}
 if(built.indices.length===0)return {body:null,exact:false}
 return {body:{...a,brep:undefined,mesh:{positions:built.positions.slice(),indices:built.indices.slice()}},exact:false}
}
