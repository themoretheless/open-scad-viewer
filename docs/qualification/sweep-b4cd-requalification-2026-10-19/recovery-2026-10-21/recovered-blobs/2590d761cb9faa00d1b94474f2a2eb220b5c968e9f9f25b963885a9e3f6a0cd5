import {inspectSolidDisplay} from './solidDiagnostics'
import type {DirectDocument} from './directModeling'
import {analyzeNurbsBrep,tessellateNurbsBrep,type BrepMassProperties} from './geometry/brep'
export type SolidBrepToolOptions={id:string;kind:'mass'}|{id:string;kind:'mesh';segments:number}|{id:string;kind:'display';offset:number;normal:number[]}
export type SolidBrepToolResult={kind:'display';id:string;diagnostics:ReturnType<typeof inspectSolidDisplay>}|{kind:'mass';id:string;mass:BrepMassProperties}|{kind:'mesh';id:string;document:DirectDocument}
export function applySolidBrepTool(source:DirectDocument,p:SolidBrepToolOptions):SolidBrepToolResult{
 const body=source.bodies.find(b=>b.id===p.id)
 if(p.kind==='display'){
  if(!body)throw Error('Select an existing body.')
  return {kind:'display',id:p.id,diagnostics:inspectSolidDisplay(body.mesh,p.offset,p.normal)}
 }
 if(!body?.brep)throw Error('Select an authored B-rep body.')
 if(p.kind==='mass')return {kind:'mass',id:p.id,mass:analyzeNurbsBrep(body.brep)}
 if(!Number.isInteger(p.segments)||p.segments<1||p.segments>32)throw Error('B-rep detail must be an integer between 1 and 32.')
 const document=structuredClone(source),target=document.bodies.find(b=>b.id===p.id)!,built=tessellateNurbsBrep(body.brep,p.segments)
 target.mesh={positions:built.positions.slice(),indices:built.indices.slice()}
 return {kind:'mesh',id:p.id,document}
}
