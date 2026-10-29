import {beforeAll,expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {runOwnNurbs} from '../src/mcp/rushGraphNurbsRuntime'
const document={language:'rush/nurbs-1',units:'mm',nodes:[
 {id:'a',op:'plane_patch',origin:[0,0,0],axis_u:[1,0,0],axis_v:[0,1,0]},
 {id:'b',op:'transform',input:'a',matrix:[[1,0,0,0],[0,1,0,0],[0,0,1,.25],[0,0,0,1]]},
],root:'b'}
const request={action:'evaluate' as const,offsetDeviation:{source:'a',candidate:'b',distance:.25,tolerance:1e-8,maxCells:1000,maxDepth:24}}
beforeAll(async()=>{await warmGeometryKernel()})
it('resolves original graph nodes into the same continuous Rust check without changing the graph',()=>{
 const before=structuredClone(document),r=buildOwnNurbs(document,request)
 expect(r).toMatchObject({ok:true,automatic_fallback:false,offsetDeviation:{decision:'within_tolerance',continuousBound:true,topologyAuthority:false}})
 expect(buildOwnNurbs(document,{...request,offsetDeviation:{...request.offsetDeviation,maxCells:0}})).toMatchObject({offsetDeviation:{decision:'unresolved',errorUpper:null}})
 expect(buildOwnNurbs(document,{...request,offsetDeviation:{...request.offsetDeviation,distance:.5}})).toMatchObject({offsetDeviation:{decision:'exceeds_tolerance',errorUpper:null}})
 expect(()=>buildOwnNurbs(document,{...request,offsetDeviation:{...request.offsetDeviation,source:'missing'}})).toThrow()
 expect(document).toEqual(before)
})
it('runs the inspection in the actual disposable Rush NURBS process',async()=>{
 expect(await runOwnNurbs(document,request)).toMatchObject({ok:true,offsetDeviation:{decision:'within_tolerance',continuousBound:true,topologyAuthority:false}})
},30_000)
