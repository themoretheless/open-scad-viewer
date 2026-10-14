import {expect,it} from 'vitest'
import {inspectSweepSeams,type SweepSeamDeclaration} from '../src/services/nurbsSweepAudit'
import type {NurbsSurface} from '../src/services/nurbsSurface'
import {trimNurbsSurface} from '../src/services/nurbsSurface'
import {lineNurbsCurve,roundPolylineNurbsCurve,transitionPolylineNurbsCurve,sweepNurbsCurve} from '../src/services/nurbsConstructors'
import {decomposeNurbsCurve} from '../src/services/nurbsCurve'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
const plane=(y:number):NurbsSurface=>({degreeU:1,degreeV:2,knotsU:[0,0,1,1],knotsV:[0,0,0,1,1,1],controlPoints:[0,1].map(x=>[0,.5,1].map(v=>[x,y+v,0])),weights:[[1,1,1],[1,1,1]]})
const seam:SweepSeamDeclaration={patches:[0,1],boundaries:['vMax','vMin'],order:2,normalScale:1,jetTolerance:1e-8}
it('preserves regularity and bounded jet evidence through WASM without promoting exact G1/G2',()=>{
 const patches=[plane(0),plane(1)],before=structuredClone(patches)
 expect(inspectSweepSeams(patches,[seam])).toMatchObject({allWithinJetBudget:true,exactG1G2Certified:false,inspectedSeams:1,seams:[{withinJetBudget:true,regularityCertified:true,tangentialSmoothnessCertified:true,reason:'jets-within-budget'}]})
 expect(patches).toEqual(before)
 expect(inspectSweepSeams(patches,[seam],0)).toMatchObject({allWithinJetBudget:false,inspectedSeams:0,seams:[{errorUpper:null,reason:'seam-budget-exhausted'}]})
 expect(()=>inspectSweepSeams(patches,[{...seam,normalScale:-1}])).toThrow()
})
it('qualifies real open corner sweeps and cyclic round seams through the public WASM API',()=>{
 const profile=lineNurbsCurve([0,0,0],[0,0,1])
 const spatial:[number,number,number][]=[[0,0,0],[10,0,2],[10,10,5],[0,10,1]]
 const planar:[number,number,number][]=[[0,0,0],[10,0,0],[10,10,0],[0,10,0]]
 for(const sites of [spatial,planar])for(const closed of [false,true])for(const order of [1,2] as const){
  const path=order===1?roundPolylineNurbsCurve(sites,2,closed):transitionPolylineNurbsCurve(sites,2,closed)
  const base=sweepNurbsCurve(profile,path),pieces=decomposeNurbsCurve(path)
  const walls=pieces.map(piece=>trimNurbsSurface(base,[0,1,...piece.domain]))
  const joins:SweepSeamDeclaration[]=[]
  const count=pieces.length-(closed?0:1)
  for(let i=0;i<count;i++){
   const next=(i+1)%pieces.length,a=pieces[i]!.domain,b=pieces[next]!.domain
   joins.push({patches:[i,next],boundaries:['vMax','vMin'],order,normalScale:(b[1]-b[0])/(a[1]-a[0]),jetTolerance:1e-8})
  }
  expect(inspectSweepSeams(walls,joins).allWithinJetBudget).toBe(true)
 }
})
it('refuses a sharp seam and bounds second-order jets of ruled walls',()=>{
 const a=plane(0),b=plane(1);for(const row of b.controlPoints)row[1]![2]=.25
 expect(inspectSweepSeams([a,b],[{...seam,order:1}]).allWithinJetBudget).toBe(false)
 const ruled=(s:NurbsSurface):NurbsSurface=>({...s,degreeV:1,knotsV:[0,0,1,1],controlPoints:s.controlPoints.map(row=>[row[0]!,row[2]!]),weights:[[1,1],[1,1]]})
 expect(inspectSweepSeams([ruled(a),ruled(plane(1))],[seam])).toMatchObject({allWithinJetBudget:true,exactG1G2Certified:false,seams:[{withinJetBudget:true,regularityCertified:true,reason:'jets-within-budget'}]})
 const bent=ruled(plane(1));for(const row of bent.controlPoints)row[1]![2]=.25
 expect(inspectSweepSeams([ruled(a),bent],[seam]).allWithinJetBudget).toBe(false)
 // Match Euclidean first jets while retaining different rational second jets.
 const rationalA=ruled(a),rationalB=ruled(plane(1))
 for(const row of rationalA.weights)row[0]=.5
 for(const row of rationalB.weights)row[1]=2
 for(const row of rationalB.controlPoints)row[1]![1]=1.25
 expect(inspectSweepSeams([rationalA,rationalB],[{...seam,order:1}]).allWithinJetBudget).toBe(true)
 expect(inspectSweepSeams([rationalA,rationalB],[seam])).toMatchObject({allWithinJetBudget:false,seams:[{reason:'jet-deviation-exceeds-budget'}]})
})
it('audits authoritative surface coefficients produced from Rush round and transition source',()=>{
 const source=readFileSync('examples/rush/transition-translation-sweep.r','utf8')
 for(const order of [1,2] as const){
  const text=order===2?source:source.replace('transition_polyline_curve','round_polyline_curve').replace('setback:','radius:')
  const graph=compileRushFrontend(text)
  const node=graph.document.nodes.find(n=>n.op==='surface_sweep')!
  const built=buildOwnNurbs(graph.document,{action:'build'})
  const surface=built.report.definitions[node.id] as unknown as NurbsSurface
  const breaks=[...new Set(surface.knotsV)]
  const walls=breaks.slice(1).map((end,i)=>trimNurbsSurface(surface,[0,1,breaks[i]!,end]))
  const joins:SweepSeamDeclaration[]=walls.slice(1).map((_,i)=>({patches:[i,i+1],boundaries:['vMax','vMin'],order,normalScale:(breaks[i+2]!-breaks[i+1]!)/(breaks[i+1]!-breaks[i]!),jetTolerance:1e-8}))
  expect(inspectSweepSeams(walls,joins).allWithinJetBudget).toBe(true)
  expect(inspectSweepSeams(walls,joins,0).allWithinJetBudget).toBe(false)
 }
})
