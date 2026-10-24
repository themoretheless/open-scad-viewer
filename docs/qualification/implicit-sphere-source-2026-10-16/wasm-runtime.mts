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
    for(const key of ['certified','solidGeometryCertified','continuousBound','sourceBoundaryExact','topologyPreserved','fullBoundaryG1Certified','fullBoundaryG2Certified','idealSourceBoundaryG1Certified','idealSourceBoundaryG2Certified','quotientVertexLinksComplete']) assert.equal(r[key],true,`${stage}/${key}: ${r.reason}`)
    assert.equal(r.boundaryErrorUpper,0)
    assert.equal(r.seamContinuity,'G2')
    assert.equal(r.regularity.allChartsRegularModuloPoles,true)
    assert.equal(r.volume.proven,true)
    assert.equal(r.volume.absenceOfSelfIntersections,true)
    assert(r.implicitFaces.every((p:any)=>p.exactMembership && p.gradientNonzero && p.gradientLower>0))
}
const cases:any[]=[]
for(const name of ['sphere-two-poles','sphere-nonbinary-radius','sphere-regular','sphere-translated-poles']) {
    const input=JSON.parse(readFileSync(`${fixturesDir}/${name}.source.json`,'utf8'))
    const built=request(input);full(built,`${name}/construct`)
    const auditInput={...input,op:'brep_implicit_sphere_source_audit',model:built.model}
    const audited=request(auditInput);full(audited,`${name}/audit`)
    assert.deepEqual(audited.model,built.model)
    const exported=request({op:'brep_nurbs_export_step_v9',model:built.model})
    assert.equal(exported.certificate.complete,true)
    const imported=request({op:'brep_nurbs_import_step_v9',text:exported.text})
    assert.equal(imported.certificate.complete,true)
    full(request({...auditInput,model:imported.model}),`${name}/STEP audit`)
    const short=request({...input,certified:true,fullBoundaryG2Certified:true,limits:{...input.limits,implicitExactWork:0}})
    assert.equal(short.certified,false);assert.equal(short.model,null);assert.equal(short.fullBoundaryG2Certified,false)
    const wrong=request({...auditInput,source:{...input.source,radius:input.source.radius*2}})
    assert.equal(wrong.certified,false);assert.equal(wrong.model,null);assert.equal(wrong.fullBoundaryG2Certified,false)
    cases.push({name,passed:true,faces:built.model.faces.length,poleEdges:built.poleEdges.length,wasmStepReaudited:true,shortBudgetRefused:true,wrongSourceRefused:true})
    process.stdout.write(`${name}: Rust WASM original global/G2, STEP re-audit and refusal passed\n`)
}
const report={schema:'original-implicit-sphere-wasm-runtime/1',wasmSha256:createHash('sha256').update(binary).digest('hex'),wasmBytes:binary.length,cases,passed:true,scope:'Four specified original implicit sphere fixtures through private release WASM ABI; no public package, browser/UI or arbitrary loft/frame/cap qualification'}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
