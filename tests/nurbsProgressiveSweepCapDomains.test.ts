import {expect,it} from 'vitest'
import {bezierNurbsCurve,inspectProgressiveSweepIdealCapDomains} from '../src/services/nurbsConstructors'
const square=(lo:number,hi:number,reverse=false)=>{
 const p=[[lo,lo,lo],[hi,lo,hi],[hi,hi,hi],[lo,hi,lo]]
 if(reverse)p.reverse()
 return p.map((v,i)=>bezierNurbsCurve([v,p[(i+1)%4]!]))
}
const path=()=>bezierNurbsCurve([[0,0,0],[0,0,10]])
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
const options={orientation:'rmf' as const,normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:33,maxDeviation:.01,
 axisScale:{degree:1,knots:[0,0,1,1],values:[[2,3,4],[2,3,4]] as [number,number,number][],weights:[1,1]}}
const budgets={tolerance:1e-9,maxPairs:10000,maxCells:100000,maxExactWork:1000000}
it('proves original oblique hollow endpoint domains independently of retained caps and body error',()=>{
 const report=inspectProgressiveSweepIdealCapDomains([...square(-2,2),...square(-1,1,true)],path(),scale,twist,options,[4,4],budgets)
 expect(report).toMatchObject({idealCapDomainsCertified:true,localDomainCertified:true,
  method:'original-progressive-endpoint-material-domains',continuousBound:false,
  retainedCapRegionsCertified:false,globalEmbeddingCertified:false,solidCertified:false})
 expect(report.endpointFrameAxes).toHaveLength(2)
})
it('discards incomplete endpoint proofs and rejects malformed loop ownership',()=>{
 const profiles=[...square(-2,2),...square(-1,1,true)]
 const proof=inspectProgressiveSweepIdealCapDomains(profiles,path(),scale,twist,options,[4,4],budgets)
 expect(proof.idealCapDomainsCertified).toBe(true)
 for(const maxCells of [0,proof.cells-1]){
  expect(inspectProgressiveSweepIdealCapDomains(profiles,path(),scale,twist,options,[4,4],{...budgets,maxCells})).toMatchObject({idealCapDomainsCertified:false,endpointFrameAxes:null})
 }
 expect(()=>inspectProgressiveSweepIdealCapDomains(profiles,path(),scale,twist,options,[7],budgets)).toThrow()
})
it('refuses touching holes and curved RMF endpoint correspondence',()=>{
 expect(inspectProgressiveSweepIdealCapDomains([...square(-2,2),...square(-2,1,true)],path(),scale,twist,options,[4,4],budgets).idealCapDomainsCertified).toBe(false)
 const curved=bezierNurbsCurve([[0,0,0],[0,0,5],[1,0,5]])
 expect(inspectProgressiveSweepIdealCapDomains(square(-2,2),curved,scale,twist,options,[4],budgets)).toMatchObject({idealCapDomainsCertified:false,endpointFrameAxes:null,reason:'endpoint-original-frame-unproved'})
})
