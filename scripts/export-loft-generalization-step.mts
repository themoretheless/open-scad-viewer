import assert from 'node:assert/strict'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import {evaluateNurbsSurface,isoNurbsCurve} from '../src/services/nurbsSurface'
import {reverseNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import type {NurbsSurface} from '../src/services/nurbsSurface'
import type {NurbsBrep} from '../src/services/geometry/brep'
import {exportDirectStepV9,importDirectStepV9} from '../src/services/cadNurbsStep'
const directory=resolve(process.argv[2]??'docs/qualification/loft-generalization-2026-10-02/step')
mkdirSync(directory,{recursive:true})
const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
function openFace(surface:NurbsSurface):NurbsBrep {
 const uv=[[0,0],[1,0],[1,1],[0,1]],curves=[isoNurbsCurve(surface,'v',0),isoNurbsCurve(surface,'u',1),reverseNurbsCurve(isoNurbsCurve(surface,'v',1)),reverseNurbsCurve(isoNurbsCurve(surface,'u',0))]
 return {vertices:uv.map(p=>({point:evaluateNurbsSurface(surface,p[0]!,p[1]!).point as [number,number,number]})),
  edges:curves.map((curve,i)=>({curve,vertices:[i,(i+1)%4],degenerate:false})),
  loops:[{coedges:curves.map((_,i)=>({edge:i,reversed:false,pcurve:line(uv[i]!,uv[(i+1)%4]!)}))}],
  faces:[{surface,outer:0,holes:[]}],shells:[{faces:[{face:0,reversed:false}],closed:false}],bodies:[],toleranceMm:1e-6}
}
const mapped=readFileSync('examples/rush/cartesian-mapped-loft.r','utf8')
const cases=[
 ['cartesian-control-tangent',readFileSync('examples/rush/cartesian-control-tangent-loft.r','utf8')],
 ['cartesian-auto-control-tangent',readFileSync('examples/rush/cartesian-auto-control-tangent-loft.r','utf8')],
 ['cartesian-nested-mapped',mapped],
 ['cartesian-auto-nested-mapped',mapped.replace('guided_loft_surface(a,b','auto_guided_loft_surface(a,b').replace('guide_parameters: [0],','').replace('error_budget:','budget:')],
 ['piecewise-mapped-natural',readFileSync('examples/rush/mapped-natural-loft.r','utf8')],
 ['authored-nonplanar-caps',readFileSync('examples/rush/authored-nonplanar-cap-loft.r','utf8')],
] as const
const reports=[]
for(const [name,source] of cases){
 const compiled=compileModelGraphText(source),built=buildOwnNurbs(compiled.document,{action:'build'})
 const definitions=Object.values(built.report.definitions) as any[]
 const retained=definitions.find(d=>d.kind==='brep')
 const definition=retained??definitions.find(d=>d.kind==='surface')
 const {kind:_,...geometry}=definition
 const model:NurbsBrep=retained?geometry as NurbsBrep:openFace(geometry as NurbsSurface)
 const exported=exportDirectStepV9(model),back=importDirectStepV9(exported.text).model
 assert.equal(back.faces.length,model.faces.length);assert.equal(back.edges.length,model.edges.length);assert.equal(back.bodies.length,model.bodies.length)
 const samples=[];let maxError=0;const used=new Set<number>();const parameterMappings=[]
 for(let face=0;face<model.faces.length;face++){
  const group=[]
  for(const u of [0,.13,.37,.83,1])for(const v of [0,.17,.37,.83,1]){
   const expected=evaluateNurbsSurface(model.faces[face]!.surface,u,v)
   group.push({face,u,v,point:expected.point,dv:expected.dv})
  }
  // The STEP solid writer reverses U on oppositely oriented face uses.
  // Preserve this explicit affine parameter change in the round-trip evidence.
  const matches=back.faces.flatMap((f,index)=>[false,true].map(reverseU=>({index,reverseU,error:Math.max(...group.flatMap(s=>{
   const actual=evaluateNurbsSurface(f.surface,reverseU?1-s.u:s.u,s.v).point
   return s.point.map((value,d)=>Math.abs(value-actual[d]!))
  }))}))).filter(m=>m.error<1e-9)
  assert.equal(matches.length,1,name+' unique carrier match')
  assert(!used.has(matches[0]!.index));used.add(matches[0]!.index)
  parameterMappings.push({sourceFace:face,importedFace:matches[0]!.index,reverseU:matches[0]!.reverseU})
  maxError=Math.max(maxError,matches[0]!.error);samples.push(...group)
 }

 writeFileSync(resolve(directory,name+'.step'),exported.text)
 writeFileSync(resolve(directory,name+'.samples.json'),JSON.stringify(samples,null,2)+'\n')
 reports.push({case:name,faces:model.faces.length,edges:model.edges.length,bodies:model.bodies.length,samples:samples.length,maxRoundTripPositionError:maxError,
  expectedVolume:retained?1:null,parameterMappings,certificate:exported.certificate,construction:built.report.construction})
}
writeFileSync(resolve(directory,'wasm-rush-round-trip.json'),JSON.stringify({scope:'finite packaged WASM/Rush/STEP cases; independent parser verification is separate',cases:reports},null,2)+'\n')
console.log(JSON.stringify(reports.map(({case:name,faces,edges,bodies,samples,maxRoundTripPositionError})=>({case:name,faces,edges,bodies,samples,maxRoundTripPositionError})),null,2))
