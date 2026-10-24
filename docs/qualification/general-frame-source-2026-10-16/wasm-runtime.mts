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
    for(const key of ['certified','solidGeometryCertified','continuousBound','withinBudget','topologyPreserved','shellRolesCertified','fullBoundaryG1Certified','fullBoundaryG2Certified','idealSourceBoundaryG1Certified','idealSourceBoundaryG2Certified']) assert.equal(r[key],true,`${stage}/${key}: ${r.reason}`)
    assert.equal(r.seamContinuity,'G2');assert.equal(r.physicalCapCount,0)
    assert.equal(r.model.faces.length,256);assert(r.boundaryErrorUpper<=0.05)
    for(const p of [r.original.volume,r.retainedVolume]) for(const key of ['embeddingCertified','volumeRegularityCertified','parameterTopologyPreserved']) assert.equal(p[key],true,`${stage}/${key}`)
}
const input=JSON.parse(readFileSync(requestPath,'utf8'))
const built=request(input);full(built,'construct')
const audited=request({...input,op:'brep_sweep_smooth_closed_material_audit',model:built.model});full(audited,'audit')
assert.deepEqual(audited.model,built.model)
const exported=request({op:'brep_nurbs_export_step_v9',model:built.model});assert.equal(exported.certificate.complete,true)
const imported=request({op:'brep_nurbs_import_step_v9',text:exported.text});assert.equal(imported.certificate.complete,true)
for(const key of ['vertices','edges','loops','faces','shells','bodies']) assert.equal(imported.model[key].length,built.model[key].length)
const restored=request({...input,op:'brep_sweep_smooth_closed_material_audit',model:imported.model});full(restored,'STEP re-audit')
assert.equal(restored.boundaryErrorUpper,built.boundaryErrorUpper)
// Negating both the original longitudinal and twist laws preserves the
// physical frame action after its own initial-coordinate transform. It also
// exercises a negative longitudinal/path dot without trusting shell flags.
const reversed=structuredClone(input)
reversed.source.frame_axis.controlPoints=reversed.source.frame_axis.controlPoints.map((p:number[])=>p.map(x=>-x))
reversed.source.twist.controlPoints=reversed.source.twist.controlPoints.map((p:number[])=>p.map(x=>-x))
const reversedBody=request(reversed);full(reversedBody,'negative longitudinal')
for(const parameter of [false,true]) {
    const refused=structuredClone(input)
    refused.model=built.model;refused.certified=true;refused.idealSourceBoundaryG2Certified=true
    if(parameter)refused.source.spacing='parameter'
    else refused.idealBody.maxCells=0
    const r=request(refused)
    assert.equal(r.certified,false);assert.equal(r.solidGeometryCertified,false);assert.equal(r.model,null)
}
writeFileSync(output,JSON.stringify({schema:'original-general-frame-private-wasm-runtime/1',wasmSha256:createHash('sha256').update(binary).digest('hex'),wasmBytes:binary.length,requestSha256:createHash('sha256').update(readFileSync(requestPath)).digest('hex'),passed:true,scope:'One exact tilted authored-frame Mobius-clock body: construct/audit/STEP re-audit and budget/parameter refusal.',negativeLongitudinalPassed:true,faces:built.model.faces.length,boundaryErrorUpper:built.boundaryErrorUpper,publicPackageQualified:false,browserQualified:false,semanticG1MatrixQualified:false,fullGoalComplete:false},null,2)+'\n')
console.log('original tilted authored-frame WASM runtime passed')
