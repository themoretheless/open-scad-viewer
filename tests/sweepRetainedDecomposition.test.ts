import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {expect,it} from 'vitest'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import {createRationalBrepSectionLoft,createProgressiveMiterBrepProfileBody} from '../src/services/geometry/brep'
import {inspectSweepRetainedCorrespondence,inspectSweepRetainedDecomposition,inspectSweepRetainedCapDecomposition} from '../src/services/sweepRetainedCorrespondence'
it('covers exactly closed periodic hollow profiles through the complete boundary constructor',async()=>{
 const outer:NurbsCurve={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:Array(6).fill(1),periodic:true}
 const hole:NurbsCurve={...structuredClone(outer),controlPoints:outer.controlPoints.slice().reverse().map(p=>p.map(x=>x*.25))}
 const loops=[[outer],[hole]],before=structuredClone(loops)
 const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const body=createProgressiveMiterBrepProfileBody(loops,[[0,0,0],[0,0,10]],scale,{...scale,values:[0,0]},
  {normal:[1,0,0],initialSteps:1,maxSteps:1,maxDeviation:.001})
 expect(body.idealCapDomains?.idealCapDomainsCertified).toBe(true)
 expect(body.retainedDecomposition?.certified).toBe(true)
 expect(body.retainedCapDecomposition?.certified).toBe(true)
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,reason:null})
 expect(loops).toEqual(before)
 const {readFileSync}=await import('node:fs')
 const {compileModelGraphText}=await import('../src/services/modelGraphText')
 const {buildOwnNurbs}=await import('../src/services/modelGraphNurbsKernel')
 const graph=compileModelGraphText(readFileSync('examples/rush/miter-periodic-hollow.r','utf8')).document
 const curves=graph.nodes.filter(n=>n.op==='curve')
 expect(curves).toHaveLength(2)
 for(const curve of curves)expect(curve).toMatchObject({periodic:true})
 const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction![node.id]).toMatchObject({continuousBound:true,boundaryCertificate:{withinBudget:true}})
 expect(JSON.parse(built.nativeGeometry!.geometryJson).sweepEvidence.boundaryCertificate.continuousBound).toBe(true)
})
it('covers an unclamped hollow source through retained walls, filled caps and the complete boundary certificate',async()=>{
 const outer:NurbsCurve={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:Array(6).fill(1),periodic:false}
 const hole:NurbsCurve={...structuredClone(outer),controlPoints:outer.controlPoints.slice().reverse().map(p=>p.map(x=>x*.25))}
 const loops=[[outer],[hole]],before=structuredClone(loops)
 const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const body=createProgressiveMiterBrepProfileBody(loops,[[0,0,0],[0,0,10]],scale,{...scale,values:[0,0]},
  {normal:[1,0,0],initialSteps:1,maxSteps:1,maxDeviation:.001})
 expect(body.idealCapDomains?.idealCapDomainsCertified).toBe(true)
 expect(body.retainedDecomposition?.certified).toBe(true)
 expect(body.retainedCapDecomposition?.certified).toBe(true)
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,reason:null})
 expect(loops).toEqual(before)
 const {readFileSync}=await import('node:fs')
 const {compileModelGraphText}=await import('../src/services/modelGraphText')
 const {buildOwnNurbs}=await import('../src/services/modelGraphNurbsKernel')
 const graph=compileModelGraphText(readFileSync('examples/rush/miter-unclamped-hollow.r','utf8')).document
 const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction![node.id]).toMatchObject({continuousBound:true,boundaryCertificate:{withinBudget:true}})
 expect(JSON.parse(built.nativeGeometry!.geometryJson).sweepEvidence.boundaryCertificate.continuousBound).toBe(true)
})
it('binds low-multiplicity source spans to actual ruled walls and refuses incomplete coverage',()=>{
 const ring=(z:number):NurbsCurve=>({degree:2,knots:[0,0,0,.25,.5,.75,1,1,1],controlPoints:[[1,0,z],[1,1,z],[-1,1,z],[-1,-1,z],[1,-1,z],[1,0,z]],weights:[1,1,1,1,1,1],periodic:false})
 const sections=[[[ring(0)]],[[ring(10)]]]
 const model=createRationalBrepSectionLoft(sections),before=structuredClone({sections,model})
 expect(inspectSweepRetainedCorrespondence(model,sections,false).exact).toBe(false)
 expect(inspectSweepRetainedDecomposition(model,sections,false,100000,1024,1)).toMatchObject({certified:false,wallErrorUpper:null})
 const result=inspectSweepRetainedDecomposition(model,sections,false)
 expect(result).toMatchObject({certified:true,inspectedFaces:4,reason:null})
 expect(result.wallErrorUpper!).toBeLessThan(1e-10)
 expect(inspectSweepRetainedDecomposition(model,sections,false,result.products-1)).toMatchObject({certified:false,wallErrorUpper:null,reason:'work-limit'})
 const shifted=structuredClone(model)
 for(const face of shifted.faces.slice(0,-2))for(const row of face.surface.controlPoints)for(const pole of row)pole[2]!+=.125
 for(const edge of shifted.edges)for(const pole of edge.curve.controlPoints)pole[2]!+=.125
 expect(inspectSweepRetainedDecomposition(shifted,sections,false).wallErrorUpper!).toBeGreaterThanOrEqual(.125)
 const wrongWeights=structuredClone(model);wrongWeights.faces[0]!.surface.weights[0]![1]!+=.125
 expect(inspectSweepRetainedDecomposition(wrongWeights,sections,false)).toMatchObject({certified:false,wallErrorUpper:null,reason:'retained-wall-domain-unproved'})
 const missing=structuredClone(model);missing.faces.splice(0,1)
 expect(inspectSweepRetainedDecomposition(missing,sections,false).wallErrorUpper).toBeNull()
 const hole=(z:number):NurbsCurve=>({...ring(z),controlPoints:ring(z).controlPoints.slice().reverse().map(p=>[p[0]!*.2,p[1]!*.2,z])})
 const hollow=[[[ring(0)],[hole(0)]],[[ring(10)],[hole(10)]]]
 const hollowModel=createRationalBrepSectionLoft(hollow)
 const hollowCaps=inspectSweepRetainedCapDecomposition(hollowModel,[hollow[0]!,hollow[1]!],DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets)
 expect(hollowCaps.certified).toBe(true)
 expect(inspectSweepRetainedCapDecomposition(hollowModel,[[hollow[0]![1]!,hollow[0]![0]!],hollow[1]!],DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets).capErrorUpper).toBeNull()
 const brokenWallEdge=structuredClone(model)
 const wallEdge=brokenWallEdge.loops[brokenWallEdge.faces[0]!.outer]!.coedges[0]!.edge
 brokenWallEdge.edges[wallEdge]!.curve.controlPoints[0]![0]!+=.125
 expect(inspectSweepRetainedDecomposition(brokenWallEdge,sections,false)).toMatchObject({certified:false,wallErrorUpper:null,reason:'retained-wall-domain-unproved'})
 const clipped=structuredClone(model)
 const clippedUses=clipped.loops[clipped.faces[0]!.outer]!.coedges
 for(const use of clippedUses)for(const p of use.pcurve.controlPoints)p[0]!*=.5
 expect(inspectSweepRetainedDecomposition(clipped,sections,false)).toMatchObject({certified:false,wallErrorUpper:null,reason:'retained-wall-domain-unproved'})
 const withHole=structuredClone(model);withHole.faces[0]!.holes.push(withHole.faces[0]!.outer)
 expect(inspectSweepRetainedDecomposition(withHole,sections,false).wallErrorUpper).toBeNull()
 const badWeights=structuredClone(model);badWeights.loops[badWeights.faces[0]!.outer]!.coedges[0]!.pcurve.weights[0]=0
 expect(inspectSweepRetainedDecomposition(badWeights,sections,false).wallErrorUpper).toBeNull()
 const missingSide=structuredClone(model);missingSide.loops[missingSide.faces[0]!.outer]!.coedges.pop()
 expect(inspectSweepRetainedDecomposition(missingSide,sections,false).wallErrorUpper).toBeNull()
 const orphanWall=structuredClone(model);orphanWall.shells[0]!.faces.shift()
 expect(inspectSweepRetainedDecomposition(orphanWall,sections,false)).toMatchObject({certified:false,wallErrorUpper:null,reason:'retained-body-face-coverage-unproved'})
 const repeatedWall=structuredClone(model);repeatedWall.shells[0]!.faces[1]=structuredClone(repeatedWall.shells[0]!.faces[0]!)
 expect(inspectSweepRetainedDecomposition(repeatedWall,sections,false).wallErrorUpper).toBeNull()
 const wrongOwners=structuredClone(model);wrongOwners.bodies[0]!.innerShells.push(wrongOwners.bodies[0]!.outerShell)
 expect(inspectSweepRetainedDecomposition(wrongOwners,sections,false).wallErrorUpper).toBeNull()
 expect({sections,model}).toEqual(before)
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const twist={...law,values:[0,0]}
 const body=createProgressiveMiterBrepProfileBody([[ring(0)]],[[0,0,0],[0,0,10]],law,twist,{normal:[1,0,0],initialSteps:1,maxSteps:1,maxDeviation:.001})
 expect(body.retainedCorrespondence.exact).toBe(false)
 expect(body.retainedDecomposition).toMatchObject({certified:true,reason:null})
 expect(body.retainedWallErrorUpper!).toBeGreaterThanOrEqual(body.retainedDecomposition!.wallErrorUpper!)
 expect(body.retainedWallErrorUpper!).toBeLessThan(.001)
 expect(body.retainedCaps?.exact).toBe(false)
 expect(body.retainedCapDecomposition).toMatchObject({certified:true,reason:null})
 expect(body.idealCapDomains?.idealCapDomainsCertified).toBe(true)
 expect(body.filledCapErrorUpper).not.toBeNull()
 expect(body.boundaryErrorWithinBudget).toBe(true)
 expect(body.boundaryErrorUpper!).toBeLessThan(.001)
 const caps=inspectSweepRetainedCapDecomposition(model,[sections[0]!,sections[1]!],DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets)
 expect(caps.certified).toBe(true)
 expect(inspectSweepRetainedCapDecomposition(model,[sections[0]!,sections[1]!],{...DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets,maxExactWork:1})).toMatchObject({certified:false,capErrorUpper:null})
 expect(caps.capErrorUpper!.every(x=>x>=0&&x<1e-10)).toBe(true)
 expect(inspectSweepRetainedCapDecomposition(model,[sections[0]!,sections[1]!],DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets,caps.products-1).capErrorUpper).toBeNull()
 const brokenCap=structuredClone(model)
 const capEdge=brokenCap.loops[brokenCap.faces.at(-1)!.outer]!.coedges[0]!.edge
 brokenCap.edges[capEdge]!.curve.controlPoints[0]![0]!+=.125
 expect(inspectSweepRetainedCapDecomposition(brokenCap,[sections[0]!,sections[1]!],DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets)).toMatchObject({certified:false,capErrorUpper:null})
 const hollowBody=createProgressiveMiterBrepProfileBody(hollow[0]!,[[0,0,0],[0,0,10]],law,twist,{normal:[1,0,0],initialSteps:1,maxSteps:1,maxDeviation:.001})
 expect(hollowBody.idealCapDomains?.idealCapDomainsCertified).toBe(true)
 expect(hollowBody.retainedCapDecomposition?.certified).toBe(true)
 expect(hollowBody.boundaryErrorWithinBudget).toBe(true)
 expect(hollowBody.approximation.report.continuousBound).toBe(false)
 const affineBody=createProgressiveMiterBrepProfileBody(hollow[0]!,[[0,0,0],[0,0,10]],law,twist,{normal:[1,0,0],initialSteps:1,maxSteps:1,maxDeviation:.001,
  axisScale:{degree:1,knots:[0,0,1,1],values:[[2,1,1],[2,1,1]],weights:[1,1]},
  centerLaw:{degree:1,knots:[0,0,1,1],values:[[.125,0,0],[.125,0,0]],weights:[1,1]},
 })
 expect(affineBody.approximation.report.affineLawsApplied).toBe(true)
 expect(affineBody.boundaryErrorWithinBudget).toBe(true)
 expect(body.approximation.report.continuousBound).toBe(false)
})

it('requires complete actual wall domains on the exact coefficient route',()=>{
 const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
 const square=(z:number)=>[[line([0,0,z],[1,0,z]),line([1,0,z],[1,1,z]),line([1,1,z],[0,1,z]),line([0,1,z],[0,0,z])]]
 const sections=[square(0),square(10)],model=createRationalBrepSectionLoft(sections)
 expect(inspectSweepRetainedCorrespondence(model,sections,false).exact).toBe(true)
 const changedSource=structuredClone(sections)
 changedSource[0]![0]![0]!.controlPoints[1]![0]=1+Number.EPSILON
 expect(inspectSweepRetainedCorrespondence(model,changedSource,false)).toMatchObject({exact:false,wallErrorUpper:null,reason:'retained-wall-mismatch'})
 const changedWeights=structuredClone(sections)
 changedWeights[0]![0]![0]!.weights[0]=.5
 expect(inspectSweepRetainedCorrespondence(model,changedWeights,false)).toMatchObject({exact:false,wallErrorUpper:null,reason:'retained-wall-mismatch'})
 const repartitioned=structuredClone(sections)
 const originalRing=repartitioned[1]![0]!
 repartitioned[1]=[originalRing.slice(0,2),originalRing.slice(2)]
 expect(inspectSweepRetainedCorrespondence(model,repartitioned,false)).toMatchObject({exact:false,wallErrorUpper:null,reason:'retained-wall-mismatch'})


 expect(inspectSweepRetainedCorrespondence(model,sections,false,1024,1)).toMatchObject({exact:false,wallErrorUpper:null})
 const wrongWorld=structuredClone(model)
 const firstUse=wrongWorld.loops[wrongWorld.faces[0]!.outer]!.coedges[0]!
 wrongWorld.edges[firstUse.edge]!.curve.controlPoints[0]![2]!+=.125
 expect(inspectSweepRetainedCorrespondence(wrongWorld,sections,false)).toMatchObject({exact:false,wallErrorUpper:null,reason:'retained-wall-domain-unproved'})
 const wrongDirection=structuredClone(model)
 const directedUse=wrongDirection.loops[wrongDirection.faces[0]!.outer]!.coedges[0]!
 directedUse.reversed=!directedUse.reversed
 expect(inspectSweepRetainedCorrespondence(wrongDirection,sections,false).exact).toBe(false)
 const wrongReference=structuredClone(model)
 const referencedUses=wrongReference.loops[wrongReference.faces[0]!.outer]!.coedges
 referencedUses[0]!.edge=referencedUses[1]!.edge
 expect(inspectSweepRetainedCorrespondence(wrongReference,sections,false).exact).toBe(false)
 const trimmed=structuredClone(model)
 for(const use of trimmed.loops[trimmed.faces[0]!.outer]!.coedges)for(const p of use.pcurve.controlPoints)p[1]!*=.5
 expect(inspectSweepRetainedCorrespondence(trimmed,sections,false)).toMatchObject({exact:false,wallErrorUpper:null,reason:'retained-wall-domain-unproved'})
 const shiftedStart=structuredClone(model),uses=shiftedStart.loops[shiftedStart.faces[0]!.outer]!.coedges
 uses.push(uses.shift()!)
 expect(inspectSweepRetainedCorrespondence(shiftedStart,sections,false).exact).toBe(true)
 uses.reverse()
 for(const use of uses){use.pcurve.controlPoints.reverse();use.reversed=!use.reversed}
 expect(inspectSweepRetainedCorrespondence(shiftedStart,sections,false).exact).toBe(true)
 const incomplete=structuredClone(model);incomplete.shells[0]!.faces.pop()
 expect(inspectSweepRetainedCorrespondence(incomplete,sections,false)).toMatchObject({exact:false,wallErrorUpper:null,reason:'retained-body-face-coverage-unproved'})
})
