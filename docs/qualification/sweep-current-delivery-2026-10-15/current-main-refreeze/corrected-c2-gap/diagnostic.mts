import {readFileSync,writeFileSync} from 'node:fs'
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
const root='/private/tmp/osv-sweep-current-main-source-20261015',out='/private/tmp/osv-sweep-corrected-c2-diagnostic-20261015.json'
const gate=JSON.parse(readFileSync('/private/tmp/osv-sweep-current-main-qualification-20261015/fresh207-ui478/receipt.json','utf8'))
assert.equal(gate.phases.at(-1).name,'step207-export');assert.equal(gate.phases.at(-1).state,'running')
const bytes=readFileSync(root+'/public/wasm/geometry-kernel.wasm'),sha=(v:Uint8Array|string)=>createHash('sha256').update(v).digest('hex')
assert.equal(sha(bytes),'562004ee79623ecb2db5bd6aef57360e8a5a5d26c21ed172e01288b23618238a')
const w=(await WebAssembly.instantiate(bytes)).instance.exports as any
const calls:any[]=[]
function request(args:any){const encoded=encodeBinary(args),p=writeLinear(w.memory,w.abi_alloc,encoded);assert.ok(p);let response:any;const begin=performance.now();try{response=decodePacked(w.memory,w.abi_free,w.abi_request(0,p,encoded.length))}finally{w.abi_free(p,encoded.length)};const milliseconds=performance.now()-begin;calls.push({args,response,milliseconds});assert.ok(milliseconds<30000);assert.ok(response.ok,JSON.stringify(response.error));return response.value}
const path={degree:3,knots:[0,0,0,0,.5,1,1,1,1],controlPoints:[[0,0,0],[.16666666666666666,0,0],[.5,.16666666666666666,0],[.8333333333333334,.6666666666666666,.5],[1,1,1.125]],weights:[1,1,1,1,1],periodic:false}
const line=(a:number[],b:number[])=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
const twist=line([0,0,0],[0,0,0]),scale=line([1,0,0],[1,0,0]),profile=line([0,.1,0],[0,.2,0])
const regularity=request({op:'sweep_frenet_frame_regularity',path,twist,maxCells:10000})
const reports:any={}
for(const orientation of ['frenet','corrected_frenet']){const result=request({op:'surface_progressive_sweep_level',profiles:[profile],path,scale,twist,normal:[0,0,1],orientation,spacing:'parameter',initial_sections:3,max_sections:65,max_deviation:10,preview_sections:17});reports[orientation]=result.report}
writeFileSync(out,JSON.stringify({schema:'corrected-c2-source-diagnostic/1',artifactSha256:sha(bytes),path,regularity,reports,calls,scope:'Diagnostic only: actual native original Frenet regularity and source-bound outcomes on one regular cubic simple-knot path. No general-mode, body, caps, Solid or UI claim.'},null,2)+'\n')
console.log(JSON.stringify({regularity,reports},null,2))
