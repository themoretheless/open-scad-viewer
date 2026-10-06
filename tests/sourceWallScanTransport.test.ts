import {expect,it} from 'vitest'
import {scanExpectation,validScan,type SourceWallScanOptions,type SourceWallScanResult} from '../src/services/sourceWallScanTransport'
const o:SourceWallScanOptions={minimumMm:7,toleranceUv:1e-7,grid:1,maxAttempts:2,limits:{cells:100,domainCells:100,normalSpans:100,maxSineSquared:1e-6}}
const key=(v:unknown)=>JSON.stringify(v)
function reply():SourceWallScanResult{return {request:structuredClone(o),thinFound:true,witness:{line:[[0,0,-1],[0,0,8]],lengthMm:[5.99,6.01],endpoints:[
 {face:0,parameter:[.1,.2],uv:[[0,1],[0,1]],worldMm:[[0,0],[0,0],[0,0]]},
 {face:1,parameter:[.8,.9],uv:[[0,1],[0,1]],worldMm:[[0,0],[0,0],[5.99,6.01]]}]},attempts:2,refused:1,facesVisited:2,facesTotal:2,proposalsExhausted:true,wholeWallQualified:false}}
it('snapshots scan requests and verifies thin witnesses, ownership and finite grid counters',()=>{
 const mutable=structuredClone(o),e=scanExpectation(mutable),r=reply();mutable.minimumMm=1;expect(e.minimumMm).toBe(7);expect(validScan(e,r,2,key)).toBe(true)
 for(const mutate of [(x:SourceWallScanResult)=>{x.thinFound=false},(x:SourceWallScanResult)=>{x.facesTotal=3},
  (x:SourceWallScanResult)=>{x.attempts=3},(x:SourceWallScanResult)=>{x.facesVisited=1},(x:SourceWallScanResult)=>{x.proposalsExhausted=false},
  (x:SourceWallScanResult)=>{x.witness!.endpoints[0].face=2},(x:SourceWallScanResult)=>{x.witness=null},
  (x:SourceWallScanResult)=>{x.request.minimumMm=1}]){const bad=reply();mutate(bad);expect(validScan(e,bad,2,key)).toBe(false)}
 expect(validScan(undefined,r,2,key)).toBe(false);expect(validScan(e,undefined,2,key)).toBe(false)
})
it('keeps incomplete and exhausted non-thin scans distinct from whole-wall qualification',()=>{
 const e={...o,maxAttempts:1},r=reply();r.request=structuredClone(e);r.attempts=1;r.facesVisited=1;r.refused=1;r.witness=null;r.thinFound=false;r.proposalsExhausted=false
 expect(validScan(e,r,2,key)).toBe(true)
 const coarse=reply();coarse.witness!.lengthMm=[14.99,15.01];coarse.thinFound=false;expect(validScan(o,coarse,2,key)).toBe(true)
 ;(coarse as unknown as {wholeWallQualified:boolean}).wholeWallQualified=true;expect(validScan(o,coarse,2,key)).toBe(false)
 for(const bad of [{...o,minimumMm:NaN},{...o,grid:0},{...o,maxAttempts:257},{...o,limits:{...o.limits,cells:0}}])expect(()=>scanExpectation(bad)).toThrow()
})
