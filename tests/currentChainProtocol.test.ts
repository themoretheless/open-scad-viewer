import {expect,it} from 'vitest'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
it('binds the diagnostic result to the selected current edge count',()=>{
 const curve={degree:1,knots:[0,0,1,2,2],weights:[1,1,1],controlPoints:[[0,0],[1,1],[2,0]]}
 const job={kind:'curveChainInspection' as const,document:{version:1 as const,sketches:[],bodies:[],curves:[{id:'a',name:'A',curve}]},ids:['a'],maxPairs:100}
 const expected=mainSolidExpectation(job)
 expect(expected).toEqual({kind:'curveChainInspection',segments:2,maxPairs:100})
 const report={scope:'represented-offset-chain',method:'outward-line-pair-interval/1',crossings:[],contacts:[],uncertain:[],degenerate:[],complete:true,checks:1,totalPairs:1,enumerationComplete:true,predicatesComplete:true,simple:true,originalOffsetTopologyCertified:false}
 expect(mainSolidResult(expected,report)).toBe(true)
 expect(mainSolidResult(expected,{...report,totalPairs:3})).toBe(false)
 expect(mainSolidResult(expected,{...report,originalOffsetTopologyCertified:true})).toBe(false)
})
