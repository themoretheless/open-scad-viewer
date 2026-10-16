// Transport/assertions only. Rust owns all geometry, certificates and refusal.
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary,decodeBinary} from '../../../src/services/valueBinaryCodec'
const [wasmPath,fixturesDir,reportPath]=process.argv.slice(2)
assert(wasmPath && fixturesDir && reportPath)
const binary=readFileSync(wasmPath)
const module=new WebAssembly.Module(binary)
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
    for(const key of ['certified','solidGeometryCertified','continuousBound','sourceBoundaryExact','topologyPreserved','absenceOfSelfIntersections','fullBoundaryG1Certified','fullBoundaryG2Certified','idealSourceBoundaryG1Certified','idealSourceBoundaryG2Certified','vertexLinksComplete','actualBoundaryRegular','exactBoundaryAgreement']) assert.equal(r[key],true,`${stage}/${key}: ${r.reason}`)
    assert.equal(r.boundaryErrorUpper,0);assert.equal(r.seamContinuity,'G2')
    assert.equal(r.originalVolume.spanwiseInteriorRegular,true)
    assert.deepEqual(r.originalVolume.knownSingularParameterEnds,[true,true])
}
const cases:any[]=[]
for(const name of ['basic','oblique-ascending','oblique-descending','three-pieces']) {
    const input=JSON.parse(readFileSync(`${fixturesDir}/${name}/source.json`,'utf8'))
    const built=request(input);full(built,`${name}/construct`)
    const auditInput={...input,op:'brep_fibered_source_audit',model:built.model}
    const audited=request(auditInput);full(audited,`${name}/audit`);assert.deepEqual(audited.model,built.model)
    const exported=request({op:'brep_nurbs_export_step_v9',model:built.model});assert.equal(exported.certificate.complete,true)
    const imported=request({op:'brep_nurbs_import_step_v9',text:exported.text});assert.equal(imported.certificate.complete,true)
    const reaudited=request({...auditInput,model:imported.model});full(reaudited,`${name}/STEP audit`);assert.deepEqual(reaudited.model,imported.model)
    const short=request({...input,certified:true,fullBoundaryG2Certified:true,limits:{...input.limits,smoothnessWork:0}})
    assert.equal(short.certified,false);assert.equal(short.model,null);assert.equal(short.fullBoundaryG2Certified,false)
    const wrongModel=structuredClone(imported.model);wrongModel.faces[0].surface.weights[1][1]*=2
    const wrong=request({...auditInput,model:wrongModel,certified:true,fullBoundaryG2Certified:true})
    assert.equal(wrong.certified,false);assert.equal(wrong.model,null);assert.equal(wrong.fullBoundaryG2Certified,false)
    cases.push({name,passed:true,faces:built.model.faces.length,wasmStepReaudited:true,shortBudgetRefused:true,wrongModelRefused:true})
    process.stdout.write(`${name}: Rust WASM original embedding, complete cap G2, STEP and refusal passed\n`)
}
const g1=JSON.parse(readFileSync(`${fixturesDir}/three-pieces/source.json`,'utf8'))
g1.source.motion[1].controls[2][0][0]=1.625
const refused=request(g1);assert.equal(refused.certified,false);assert.equal(refused.model,null)
g1.limits.order=1;const one=request(g1);assert.equal(one.certified,true);assert.equal(one.fullBoundaryG1Certified,true);assert.equal(one.fullBoundaryG2Certified,false);assert.equal(one.seamContinuity,'G1')
const zero=request({...g1,limits:{...g1.limits,order:0}});assert.equal(zero.certified,true);assert.equal(zero.fullBoundaryG1Certified,false);assert.equal(zero.seamContinuity,'C0')
const report={schema:'original-fibered-source-wasm-runtime/1',wasmSha256:createHash('sha256').update(binary).digest('hex'),wasmBytes:binary.length,cases,strictG2RefusedTrueG1:true,explicitC0Preserved:true,passed:true,scope:'Four specified original rational affine fiber fixtures through private release WASM ABI; no public package, browser/UI or arbitrary frame/cap qualification'}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
