import {expect,it,vi} from 'vitest'
const native=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/geometry/nurbs',()=>({callNurbsRust:native}))
import {inspectCurrentCurveChain} from '../src/services/inspectCurrentCurveChain'
const curve={degree:1,knots:[0,0,1,1],weights:[1,1],controlPoints:[[0,0],[2,0]]}
const document={version:1 as const,bodies:[],sketches:[],curves:[{id:'a',name:'A',curve}]}
it('passes current geometry to the kernel and rejects stale selection or malformed results',()=>{
 const report={scope:'represented-offset-chain',method:'outward-line-pair-interval/1',crossings:[],contacts:[],uncertain:[],degenerate:[],complete:true,checks:0,totalPairs:0,enumerationComplete:true,predicatesComplete:true,simple:true,originalOffsetTopologyCertified:false}
 native.mockReturnValue(report)
 expect(inspectCurrentCurveChain(document,['a'])).toEqual(report)
 expect(native).toHaveBeenLastCalledWith('curve_chain_diagnostics',{curves:[curve],maxPairs:1000000})
 expect(()=>inspectCurrentCurveChain(document,['missing'])).toThrow('no longer exists')
 expect(()=>inspectCurrentCurveChain(document,['a','a'])).toThrow('repeated objects')
 native.mockReturnValue({...report,totalPairs:1});expect(()=>inspectCurrentCurveChain(document,['a'])).toThrow('Invalid current')
})
