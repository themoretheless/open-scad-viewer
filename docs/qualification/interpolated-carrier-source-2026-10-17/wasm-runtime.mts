// ABI transport and assertions only. All geometry/proofs/STEP remain in Rust.
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary,decodeBinary} from '../../../src/services/valueBinaryCodec'
const [wasmPath,requestPath,output]=process.argv.slice(2)
assert(wasmPath && requestPath && output)
const binary=readFileSync(wasmPath),module=new WebAssembly.Module(binary)
assert.equal(WebAssembly.Module.imports(module).length,0)
const e=new WebAssembly.Instance(module).exports as any
function request(input:unknown):any {
    const bytes=encodeBinary(input),ptr=e.abi_alloc(bytes.length)
    assert(ptr)
    new Uint8Array(e.memory.buffer,ptr,bytes.length).set(bytes)
    let responsePtr=0,responseLen=0
    try {
        const packed=e.abi_request(0,ptr,bytes.length) as bigint
        responsePtr=Number(packed&0xffffffffn);responseLen=Number(packed>>32n)
        const envelope=decodeBinary(new Uint8Array(e.memory.buffer,responsePtr,responseLen).slice()) as any
        assert.equal(envelope.ok,true,JSON.stringify(envelope.error))
        return envelope.value
    } finally {
        if(responsePtr)e.abi_free(responsePtr,responseLen)
        e.abi_free(ptr,bytes.length)
    }
}
function full(r:any,stage:string) {
    for(const key of ['certified','solidGeometryCertified','continuousBound','idealBodyEmbeddingCertified']) assert.equal(r[key],true,`${stage}/${key}: ${r.reason}`)
    for(const key of ['embeddingCertified','volumeRegularityCertified','parameterTopologyPreserved']) assert.equal(r.idealBody[key],true,`${stage}/${key}`)
    assert.equal(r.seamContinuity,'C0');assert.equal(r.smoothnessCertified,false)
    assert.equal(r.model.faces.length,6);assert(r.boundaryErrorUpper<=0.01)
}
const input=JSON.parse(readFileSync(requestPath,'utf8'))
const before=structuredClone(input),built=request(input);full(built,'construct');assert.deepEqual(input,before)
const exported=request({op:'brep_nurbs_export_step_v9',model:built.model});assert.equal(exported.certificate.complete,true)
const imported=request({op:'brep_nurbs_import_step_v9',text:exported.text});assert.equal(imported.certificate.complete,true)
for(const key of ['vertices','edges','loops','faces','shells','bodies']) assert.equal(imported.model[key].length,built.model[key].length)
const tight=structuredClone(input);tight.idealBody.maxCells=built.idealBody.cells;tight.idealBody.maxExactWork=built.idealBody.exactWork
full(request(tight),'exact joint budgets')
for(const [field,value] of [['maxCells',0],['maxCells',built.idealBody.cells-1],['maxExactWork',0],['maxExactWork',built.idealBody.exactWork-1]] as const) {
    const short=structuredClone(tight);short.idealBody[field]=value;short.model=built.model;short.certified=true;short.idealBodyEmbeddingCertified=true
    const r=request(short);assert.equal(r.certified,false);assert.equal(r.idealBodyEmbeddingCertified,false);assert.equal(r.model,null)
}
writeFileSync(output,JSON.stringify({schema:'original-interpolated-carrier-private-wasm-runtime/1',wasmSha256:createHash('sha256').update(binary).digest('hex'),wasmBytes:binary.length,requestSha256:createHash('sha256').update(readFileSync(requestPath)).digest('hex'),passed:true,scope:'One open original rational carrier interpolation on independent UV domains: Solid, native STEP, exact/zero/one-short joint ledgers and forged flag refusals.',faces:built.model.faces.length,boundaryErrorUpper:built.boundaryErrorUpper,idealBody:built.idealBody,seamContinuity:built.seamContinuity,smoothnessCertified:built.smoothnessCertified,publicPackageQualified:false,browserQualified:false,semanticG1MatrixQualified:false,fullGoalComplete:false},null,2)+'\n')
console.log('original interpolated carrier WASM runtime passed')
