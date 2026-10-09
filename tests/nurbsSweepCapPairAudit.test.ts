import {expect,it} from 'vitest'
import {inspectSweepCapPairs} from '../src/services/nurbsSweepCapContacts'
import {createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
const loops=(z:number)=>[[circleNurbsCurve([0,0,z],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,z],[0,0,1],.2))]]
const budgets={clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:1000,maxPairCells:1000}
it('qualifies actual nonunit cap charts and preserves face IDs on unresolved pairs',()=>{
 const model=createRationalBrepSectionLoft([loops(0),loops(5),loops(10)]),before=structuredClone(model),caps=[16,17]
 const report=inspectSweepCapPairs(model,caps,budgets)
 expect(report).toMatchObject({capFaces:caps,unresolvedCapFaces:[],unresolvedPairs:[],globalEmbeddingCertified:false,audit:{chartsAndPairsCertified:true,pairs:{allPairsSeparated:true,separatedPairs:1}}})
 const exhausted=inspectSweepCapPairs(model,caps,{...budgets,maxPairs:0})
 expect(exhausted.audit.chartsAndPairsCertified).toBe(false)
 expect(exhausted.unresolvedPairs.map(pair=>pair.faces)).toEqual([caps])
 const noCharts=inspectSweepCapPairs(model,caps,{...budgets,maxInjectivityCells:0})
 expect(noCharts.unresolvedCapFaces).toEqual(caps)
 expect(noCharts.audit.chartsAndPairsCertified).toBe(false)
 const overlapping=structuredClone(model)
 overlapping.faces[17]!.surface=structuredClone(overlapping.faces[16]!.surface)
 const contact=inspectSweepCapPairs(overlapping,caps,{...budgets,maxPairCells:16})
 expect(contact.audit.pairs.allPairsSeparated).toBe(false)
 expect(contact.unresolvedPairs.map(pair=>pair.faces)).toEqual([caps])
 expect(()=>inspectSweepCapPairs(model,[16,16],budgets)).toThrow()
 expect(()=>inspectSweepCapPairs(model,caps,budgets,()=>{throw new DOMException('cancel','AbortError')})).toThrow(/cancel/)
 expect(model).toEqual(before)
})
