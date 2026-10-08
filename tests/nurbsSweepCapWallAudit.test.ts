import {expect,it} from 'vitest'
import {inspectSweepCapWalls,inspectSweepCoedgeAgreement,inspectSweepCoedgeExact,inspectSweepBoundaryCoverage} from '../src/services/nurbsSweepAudit'
import {inspectSweepCapContacts,inspectNativeSweepCapContacts} from '../src/services/nurbsSweepCapContacts'
import {createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
const loops=(z:number)=>[[circleNurbsCurve([0,0,z],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,z],[0,0,1],.2))]]
it('excludes all stored hollow-loft wall interiors against each coordinate-plane cap',()=>{
 const model=createRationalBrepSectionLoft([loops(0),loops(5),loops(10)]),before=structuredClone(model)
 const caps=model.faces.filter(face=>face.surface.degreeU===1&&face.surface.degreeV===1)
 expect(caps).toHaveLength(2)
 expect(caps.every(cap=>cap.surface.knotsU[0]!<0)).toBe(true)
 const walls=model.faces.filter(face=>face.surface.degreeU===2).map(face=>face.surface)
 expect(walls).toHaveLength(16)
 for(const cap of caps){
  const z=cap.surface.controlPoints[0]![0]![2]!
  const boundaries=walls.map(wall=>wall.controlPoints.every(row=>row[0]![2]===z)?'vMin' as const:wall.controlPoints.every(row=>row.at(-1)![2]===z)?'vMax' as const:null)
  const report=inspectSweepCapWalls(cap.surface,walls,boundaries,16)
  expect(report).toMatchObject({allWallInteriorsExcluded:true,inspectedWalls:16,planeAxis:2,boundaryOwnershipCertified:false,globalEmbeddingCertified:false,unresolvedWalls:[]})
  expect(report.separatedWalls).toHaveLength(8)
  expect(report.boundaryRestrictedWalls).toHaveLength(8)
  const partial=inspectSweepCapWalls(cap.surface,walls,boundaries,2)
  expect(partial).toMatchObject({allWallInteriorsExcluded:false,inspectedWalls:2,unresolvedWalls:Array.from({length:14},(_,i)=>i+2),reason:'cap-wall-budget-exhausted'})
  expect(inspectSweepCapWalls(cap.surface,walls,walls.map(()=>null),16).allWallInteriorsExcluded).toBe(false)
 }
 const capFaces=model.faces.map((face,id)=>({face,id})).filter(({face})=>face.surface.degreeU===1&&face.surface.degreeV===1).map(({id})=>id)
 const derived=inspectSweepCapContacts(model,capFaces,16)
 expect(derived).toHaveLength(2)
 const diagnosticWork=(report:typeof derived[number])=>report.capCoedges.reduce((sum,use)=>sum+use.exact.work+use.wallAgreements.reduce((n,wall)=>n+wall.exact.work,0),0)
 expect(derived.reduce((sum,report)=>sum+report.native.exactWork,0)).toBeLessThanOrEqual(1000000)
 expect(derived.reduce((sum,report)=>sum+diagnosticWork(report),0)).toBeLessThanOrEqual(1000000)
 const firstNativeBudget=derived[0]!.native.exactWork
 const shared=inspectSweepCapContacts(model,capFaces,16,()=>{},{tolerance:1e-9,maxProducts:100000,maxWork:firstNativeBudget})
 expect(shared[0]!.native.capCertified).toBe(true)
 expect(shared[1]!.native.capCertified).toBe(false)
 expect(shared[1]!.native.exactWork).toBe(0)
 expect(shared.reduce((sum,report)=>sum+report.native.exactWork,0)).toBe(firstNativeBudget)
 expect(shared.reduce((sum,report)=>sum+diagnosticWork(report),0)).toBeLessThanOrEqual(firstNativeBudget)
 const firstDiagnosticBudget=diagnosticWork(derived[0]!)
 const diagnosticShared=inspectSweepCapContacts(model,capFaces,16,()=>{},{tolerance:1e-9,maxProducts:100000,maxWork:firstDiagnosticBudget})
 expect(diagnosticShared[0]!.capAndWallExactIdentityCertified).toBe(true)
 expect(diagnosticShared[1]!.capAndWallExactIdentityCertified).toBe(false)
 expect(diagnosticWork(diagnosticShared[1]!)).toBe(0)
 expect(derived.every(report=>report.native.capCertified&&report.native.allCapWallContactsCertified&&!report.native.globalEmbeddingCertified)).toBe(true)
 expect(derived.every(report=>report.native.allowedBoundaries.length===8&&report.native.separatedWalls.length===8&&report.native.unresolvedWalls.length===0)).toBe(true)
 expect(derived.every(report=>report.capEdgeAgreementWithinBudget&&report.pairedOppositeCoedges&&report.wallEdgeAgreementWithinTolerance)).toBe(true)
 expect(derived.every(report=>report.capCoedges.length===8&&report.capCoedges.every(use=>use.agreement.errorUpper!==null&&use.agreement.errorUpper<=1e-9&&use.wallFaces.length===1))).toBe(true)
 const nativeUse=derived[0]!.capCoedges[0]!.wallAgreements[0]!,nativeCoedge=model.loops[nativeUse.wire]!.coedges[nativeUse.coedge]!
 expect(()=>inspectSweepCoedgeAgreement(model.faces[nativeUse.face]!.surface,model.edges[nativeCoedge.edge]!.curve,nativeCoedge.pcurve,{reversed:nativeCoedge.reversed,tolerance:1e-9,maxCells:100001})).toThrow()
 expect(()=>inspectSweepCoedgeAgreement(model.faces[nativeUse.face]!.surface,model.edges[nativeCoedge.edge]!.curve,nativeCoedge.pcurve,{reversed:nativeCoedge.reversed,tolerance:0,maxCells:0})).toThrow()
 expect(derived.every(report=>report.capCoedges.every(use=>use.wallAgreements[0]!.exact.exactIdentityCertified))).toBe(true)
 expect(derived.every(report=>report.capAndWallExactIdentityCertified&&report.wallBoundariesCovered)).toBe(true)
 const exactBudget=inspectSweepCapContacts(model,capFaces,16,()=>{},{tolerance:1e-9,maxProducts:100000,maxWork:0})
 expect(exactBudget.every(report=>!report.capAndWallExactIdentityCertified&&report.capCoedges.every(use=>use.exact.status==='unresolved'&&use.wallAgreements.every(wall=>wall.exact.status==='unresolved')))).toBe(true)
 const partialCoverage=structuredClone(model),partialUse=derived[0]!.capCoedges[0]!.wallAgreements[0]!
 const partialPcurve=partialCoverage.loops[partialUse.wire]!.coedges[partialUse.coedge]!.pcurve
 const declaredBoundary=derived[0]!.boundaries[derived[0]!.wallFaces.indexOf(partialUse.face)]!
 partialPcurve.controlPoints=partialPcurve.controlPoints.map(point=>[point[0]!*.5,point[1]!])
 expect(inspectSweepBoundaryCoverage(partialCoverage.faces[partialUse.face]!.surface,partialPcurve,declaredBoundary).wholeBoundaryCovered).toBe(false)
 expect(()=>inspectSweepCapContacts(partialCoverage,capFaces,16)).toThrow()
 const zeroWallBudget=inspectSweepCapContacts(model,capFaces,16,()=>{},{tolerance:1e-9,maxProducts:100000,maxCells:0})
 expect(zeroWallBudget.every(report=>!report.wallEdgeAgreementWithinTolerance&&report.capCoedges.every(use=>use.wallAgreements[0]!.agreement.status==='unresolved'))).toBe(true)
 const alteredWall=structuredClone(model),firstWallUse=derived[0]!.capCoedges[0]!.wallAgreements[0]!
 const wallPcurve=alteredWall.loops[firstWallUse.wire]!.coedges[firstWallUse.coedge]!.pcurve
 wallPcurve.controlPoints=wallPcurve.controlPoints.map(point=>[point[0]!,point[1]===0?.1:.9])
 expect(()=>inspectSweepCapContacts(alteredWall,capFaces,16)).toThrow()
 const exhausted=inspectSweepCapContacts(model,capFaces,16,()=>{},{tolerance:1e-9,maxProducts:9})
 expect(exhausted.every(report=>!report.capEdgeAgreementWithinBudget&&report.capCoedges.filter(use=>use.agreement.withinBudget).length===1)).toBe(true)
 const altered=structuredClone(model),cap=altered.faces[capFaces[0]!]!,coedge=altered.loops[cap.outer]!.coedges[0]!
 coedge.pcurve.controlPoints[1]=coedge.pcurve.controlPoints[1]!.map((value,axis)=>(value+(axis===0?(cap.surface.knotsU[1]!+cap.surface.knotsU[2]!)/2:(cap.surface.knotsV[1]!+cap.surface.knotsV[2]!)/2))/2)
 expect(()=>inspectSweepCapContacts(altered,capFaces,16)).toThrow()
 const wrongOrientation=structuredClone(model)
 wrongOrientation.loops[wrongOrientation.faces[capFaces[0]!]!.outer]!.coedges[0]!.reversed=!wrongOrientation.loops[wrongOrientation.faces[capFaces[0]!]!.outer]!.coedges[0]!.reversed
 expect(()=>inspectSweepCapContacts(wrongOrientation,capFaces,16)).toThrow()
 expect(derived.every(report=>report.audit.allWallInteriorsExcluded&&!report.audit.boundaryOwnershipCertified)).toBe(true)
 expect(derived.every(report=>report.boundaries.filter(boundary=>boundary!==null).length===8)).toBe(true)
 const flippedShellFace=structuredClone(model)
 const shellUse=flippedShellFace.shells.flatMap(shell=>shell.faces).find(use=>use.face===capFaces[0])!
 shellUse.reversed=!shellUse.reversed
 expect(()=>inspectSweepCapContacts(flippedShellFace,capFaces,16)).toThrow()
 const duplicated=structuredClone(model),duplicateCap=duplicated.faces[capFaces[0]!]!
 duplicated.loops[duplicateCap.outer]!.coedges.push(structuredClone(duplicated.loops[duplicateCap.outer]!.coedges[0]!))
 expect(()=>inspectSweepCapContacts(duplicated,capFaces,16)).toThrow()
 expect(()=>inspectSweepCapContacts(model,capFaces,16,()=>{},{tolerance:1e-9,maxProducts:-1})).toThrow(/budget/)
 const combinedBudgets={maxWalls:16,maxExactWork:1000000,maxChartCells:1000,maxTrimPairs:100000,maxTrimCells:100000,maxTrimDomainCells:1000000}
 const capTinyChange=structuredClone(model),capTiny=capTinyChange.faces[capFaces[0]!]!,tinyUv=capTinyChange.loops[capTiny.outer]!.coedges[0]!.pcurve
 tinyUv.controlPoints[1]![0]-=Number.EPSILON
 expect(inspectNativeSweepCapContacts(capTinyChange,capFaces[0]!,capFaces,combinedBudgets).allCapWallContactsCertified).toBe(false)
 const combinedPartial=inspectNativeSweepCapContacts(model,capFaces[0]!,capFaces,{...combinedBudgets,maxWalls:2})
 expect(combinedPartial).toMatchObject({capCertified:true,allCapWallContactsCertified:false,unresolvedWalls:Array.from({length:14},(_,i)=>i+2),reason:'cap-wall-budget-exhausted'})
 const partial=inspectSweepCapContacts(model,capFaces,0)
 expect(partial.every(report=>report.audit.unresolvedWalls.length===16)).toBe(true)
 expect(()=>inspectSweepCapContacts(model,[capFaces[0]!,capFaces[0]!],16)).toThrow()
 let checks=0
 expect(()=>inspectSweepCapContacts(model,capFaces,16,()=>{if(++checks===2)throw new DOMException('cancel','AbortError')})).toThrow(/cancel/)
 expect(model).toEqual(before)
})

it('distinguishes exact coedge identity from agreement within tolerance',()=>{
 const surface={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false}
 const world={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[1,0,0]],weights:[1,1],periodic:false}
 const uv={degree:1,knots:[0,0,1,1],controlPoints:[[0,0],[1,0]],weights:[1,1],periodic:false}
 expect(inspectSweepCoedgeExact(surface,world,uv,{reversed:false,maxWork:100000})).toMatchObject({status:'equal',exactIdentityCertified:true,globalEmbeddingCertified:false})
 expect(inspectSweepCoedgeExact(surface,world,uv,{reversed:false,maxWork:0})).toMatchObject({status:'unresolved',exactIdentityCertified:false,work:0})
 const changed=structuredClone(world)
 changed.controlPoints[0]![2]=1e-12
 expect(inspectSweepCoedgeAgreement(surface,changed,uv,{reversed:false,tolerance:1e-9,maxCells:100})).toMatchObject({withinTolerance:true})
 expect(inspectSweepCoedgeExact(surface,changed,uv,{reversed:false,maxWork:100000})).toMatchObject({status:'different',exactIdentityCertified:false})
 const reversed=reverseNurbsCurve(world)
 expect(inspectSweepCoedgeExact(surface,reversed,uv,{reversed:true,maxWork:100000})).toMatchObject({status:'equal',exactIdentityCertified:true})
 const outside=structuredClone(uv);outside.controlPoints[1]![0]=2
 expect(inspectSweepCoedgeExact(surface,world,outside,{reversed:false,maxWork:100000})).toMatchObject({status:'unsupported',exactIdentityCertified:false})
})

it('proves whole boundary image for rational traversal and both directions',()=>{
 const surface={degreeU:1,degreeV:1,knotsU:[-2,-2,3,3],knotsV:[0,0,1,1],controlPoints:[[[-2,0,0],[-2,0,1]],[[3,0,0],[3,0,1]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false}
 const uv={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[-2,0],[2,0],[3,0]],weights:[1,3,2],periodic:false}
 for(const curve of [uv,reverseNurbsCurve(uv)])expect(inspectSweepBoundaryCoverage(surface,curve,'vMin')).toMatchObject({wholeBoundaryCovered:true,injectivityCertified:false,boundaryOwnershipCertified:false,globalEmbeddingCertified:false})
 const backtracking={degree:5,knots:[0,0,0,0,0,0,1,1,1,1,1,1],controlPoints:[[-2,0],[3,0],[3,0],[-2,0],[-2,0],[3,0]],weights:[1,1,1,1,1,1],periodic:false}
 expect(inspectSweepBoundaryCoverage(surface,backtracking,'vMin')).toMatchObject({wholeBoundaryCovered:true,injectivityCertified:false})
 const partial=structuredClone(uv);partial.controlPoints[2]![0]=2.5
 expect(inspectSweepBoundaryCoverage(surface,partial,'vMin').wholeBoundaryCovered).toBe(false)
 const moved=structuredClone(uv);moved.controlPoints[1]![1]=Number.EPSILON
 expect(inspectSweepBoundaryCoverage(surface,moved,'vMin').wholeBoundaryCovered).toBe(false)
 expect(inspectSweepBoundaryCoverage(surface,uv,'vMax').wholeBoundaryCovered).toBe(false)
})
