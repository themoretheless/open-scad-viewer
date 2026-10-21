import {test} from 'node:test'
import assert from 'node:assert/strict'
import {EventEmitter} from 'node:events'
import {createNativeCancelProbe} from './sweep-native-cancel-probe.mjs'
// Minimal export section: the probe authenticates the complete bytes and
// resolves the exported ABI index. No geometry implementation is mocked into
// the browser qualification; these tests cover the observer's state machine.
const wasm=Buffer.from([0,97,115,109,1,0,0,0,7,15,1,11,...Buffer.from('abi_request'),0,0])
function fixture({actual=wasm,idle=false,samples=6,late=false}={}){
 const root=new EventEmitter(),calls=[];let fetched=false
 const parsed=()=>root.emit('Target.receivedMessageFromTarget',{sessionId:'s',message:JSON.stringify({method:'Debugger.scriptParsed',params:{scriptId:'geometry',scriptLanguage:'WebAssembly',url:'wasm://geometry'}})})
 root.send=async(method,params)=>{
  if(method==='Target.getTargets')return {targetInfos:[{targetId:'w',type:'worker',url:'geometry.worker-test.js'}]}
  if(method==='Target.attachToTarget')return {sessionId:'s'}
  if(method!=='Target.sendMessageToTarget')return {}
  const m=JSON.parse(params.message);calls.push(m.method);let result={}
  if(m.method==='Debugger.enable'&&!late)parsed()
  if(m.method==='Debugger.getScriptSource'){fetched=true;result={bytecode:actual.toString('base64')}}
  if(m.method==='Profiler.start'&&late&&!fetched)parsed()
  if(m.method==='Profiler.stop')result={profile:{startTime:0,endTime:(samples+1)*1000,nodes:[{id:1,callFrame:{functionName:'wasm-function[0]',scriptId:'geometry',url:'wasm://geometry'}},{id:2,callFrame:{functionName:'(idle)',scriptId:'',url:''}}],samples:[...Array(samples).fill(1),...(idle?[2]:[])],timeDeltas:Array(samples+(idle?1:0)).fill(1000)}}
  queueMicrotask(()=>root.emit('Target.receivedMessageFromTarget',{sessionId:'s',message:JSON.stringify({id:m.id,result})}));return {}
 }
 root.detach=async()=>{}
 return {browser:{newBrowserCDPSession:async()=>root},calls}
}
test('authenticates a warm module before profiling and accepts its first live ABI tail',async()=>{
 const f=fixture(),p=await createNativeCancelProbe(f.browser,wasm)
 try{await p.attachExisting();await p.armRunningGeometry();const r=await p.observeRunningGeometry();assert.equal(r.nativeSampleCount,6);assert.equal(r.debuggerPaused,false);assert.ok(f.calls.indexOf('Debugger.getScriptSource')<f.calls.indexOf('Profiler.start'));assert.equal(f.calls.filter(x=>x==='Profiler.stop').length,1);assert.ok(!f.calls.includes('Debugger.pause'))}finally{await p.close()}
})
test('a cold module authenticated after profiling needs a new live sample',async()=>{
 const f=fixture({late:true}),p=await createNativeCancelProbe(f.browser,wasm)
 try{await p.attachExisting();const r=await p.observeRunningGeometry();assert.equal(r.nativeSampleCount,6);assert.equal(f.calls.filter(x=>x==='Profiler.stop').length,2)}finally{await p.close()}
})
for(const [name,options] of [['foreign bytes',{actual:Buffer.from([...wasm,1])}],['idle tail',{idle:true}],['fewer than five native samples',{samples:4}]])test(`refuses ${name}`,async()=>{
 const f=fixture(options),p=await createNativeCancelProbe(f.browser,wasm)
 try{await p.attachExisting();await assert.rejects(p.observeRunningGeometry(),/No running geometry ABI/)}finally{await p.close()}
})
