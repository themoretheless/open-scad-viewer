import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {encodeBinary,decodeBinary} from '../src/app/valueBinaryCodec.ts'
const [path,directory]=process.argv.slice(2);assert(path&&directory)
mkdirSync(directory,{recursive:true})
const bytes=readFileSync(path),module=await WebAssembly.compile(bytes)
assert.deepEqual(WebAssembly.Module.imports(module),[])
const instance=await WebAssembly.instantiate(module,{})
const wasm=instance.exports as unknown as {memory:WebAssembly.Memory;abi_alloc(n:number):number;abi_free(p:number,n:number):void;abi_request(op:number,p:number,n:number):bigint}
function request(value:unknown):any{
 const data=encodeBinary(value),ptr=wasm.abi_alloc(data.length);assert(ptr)
 new Uint8Array(wasm.memory.buffer,ptr,data.length).set(data)
 try{const packed=wasm.abi_request(0,ptr,data.length),p=Number(packed&0xffffffffn),n=Number(packed>>32n)
  try{return decodeBinary(new Uint8Array(wasm.memory.buffer,p,n))}finally{wasm.abi_free(p,n)}
 }finally{wasm.abi_free(ptr,data.length)}
}
const source=request({op:'brep_nurbs_box',min:[0,0,0],max:[4,8,6]});assert(source.ok)
const face=(axis:number,value:number)=>source.value.faces.findIndex((f:any)=>f.surface.controlPoints.flat().every((p:number[])=>p[axis]===value))
const faces=[face(0,0),face(1,8),face(0,4)];assert(faces.every(f=>f>=0))
const carrier=(rational:boolean)=>({space:'d3',geometryKind:'solid',representation:rational?'rational-brep':'analytic-brep',evidence:{tag:'representation-preserving'}})
const cube={id:0,kind:'box',valueType:carrier(false),size:[4,8,6],center:false}
const round={id:1,kind:'full-round-exact',input:0,faces,valueType:carrier(true)}
const graph=(nodes:unknown[])=>({op:'brep_graph',nodes,outputs:[nodes.length-1],maxNodes:4,maxBytes:4194304})
const cases:any[]=[]
function run(name:string,value:unknown,ok:boolean):any{const start=performance.now(),reply=request(value)
 assert.equal(reply.ok,ok,`${name}: ${JSON.stringify(reply.error)}`)
 cases.push({name,ok,elapsedMs:performance.now()-start,error:reply.error});console.log(JSON.stringify(cases.at(-1)));return reply.value}
const result=run('public-graph-exact-result',graph([cube,round]),true)
const value=result.outcomes[0].geometry
assert.equal(value.representation,'rational-brep')
assert.deepEqual(value.model,value.artifact.geometry)
assert.equal(typeof value.model.vertices[0].point[0][0],'string')
assert.equal(value.model.faces.length,value.model.topologyIds.faces.length)
run('full-artifact-character-limit',{...graph([cube,round]),maxCharacters:JSON.stringify(value.artifact.source).length+JSON.stringify(value.model).length+1024},false)
run('wrong-output-carrier',graph([cube,{...round,valueType:carrier(false)}]),false)
run('legacy-primitive-cannot-claim-rational',graph([{...cube,valueType:carrier(true)}]),false)
run('material-budget-refusal',graph([cube,{...round,limits:{maxPairProjections:0}}]),false)
run('no-legacy-transform-cast',graph([cube,round,{id:2,kind:'transform',input:1,valueType:carrier(false),matrix:[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]}]),false)
const report={schema:'full-round-exact-native-graph-wasm/1',passed:true,cases,
 wasmPath:resolve(path),wasmSha256:createHash('sha256').update(bytes).digest('hex'),wasmBytes:bytes.length,
 scope:'Public native DAG via actual WASM ABI. High-level ModelGraph/semantic frontend schemas and packaged app kernel are not qualified.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,wasmSha256:report.wasmSha256}))
