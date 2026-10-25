// Transport and assertions only. All geometry/certificate decisions run in Rust.
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
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
    for(const key of ['certified','solidGeometryCertified','continuousBound','topologyPreserved','shellRolesCertified','fullBoundaryG1Certified','fullBoundaryG2Certified','idealSourceBoundaryG1Certified','idealSourceBoundaryG2Certified','physicalCapsCertified']) assert.equal(r[key],true,`${stage}/${key}: ${r.reason}`)
    assert.equal(r.physicalCapCount,2);assert.equal(r.boundaryErrorUpper,0);assert.equal(r.filledCapErrorUpper,0)
    assert.equal(r.seamContinuity,'G2');assert.equal(r.caps.length,2)
    assert(r.caps.every((p:any)=>p.faces.length===4 && p.parameterMonotonicityCertified))
}
const cases:any[]=[]
for(const name of ['planar-caps','nonplanar-caps','moving-affine-caps','reflected-caps','rational-profile-caps','rational-profile-affine-caps']) {
    const input=JSON.parse(readFileSync(`${fixturesDir}/${name}.source.json`,'utf8'))
    // STEP artifacts retain the audit's zero reconstruction allowance. Runtime
    // construction deliberately supplies its own explicit owner allowance.
    input.limits.constructionRows=1000000
    const built=request(input);full(built,`${name}/construct`)
    const auditInput={...input,op:'brep_rational_closed_material_audit',model:built.model,limits:{...input.limits,constructionRows:0}}
    const audited=request(auditInput);full(audited,`${name}/audit`)
    assert.equal(audited.sourceWork.constructionRows,0)
    assert.deepEqual(audited.model,built.model)
    const exported=request({op:'brep_nurbs_export_step_v9',model:built.model})
    assert.equal(exported.certificate.complete,true)
    const imported=request({op:'brep_nurbs_import_step_v9',text:exported.text})
    assert.equal(imported.certificate.complete,true)
    const reaudited=request({...auditInput,model:imported.model});full(reaudited,`${name}/wasm-step-audit`)
    assert.equal(reaudited.sourceWork.constructionRows,0)
    const short=request({...input,certified:true,fullBoundaryG2Certified:true,limits:{...input.limits,maxBindingWork:0}})
    assert.equal(short.certified,false);assert.equal(short.model,null);assert.equal(short.fullBoundaryG2Certified,false)
    cases.push({name,passed:true,faces:built.model.faces.length,physicalCaps:built.physicalCapCount,wasmStepReaudited:true,shortBudgetRefused:true})
    process.stdout.write(`${name}: WASM original/actual G2, cap/volume/STEP audit and short-budget refusal passed\n`)
}
const input=JSON.parse(readFileSync(`${fixturesDir}/nonplanar-caps.source.json`,'utf8'));input.limits.constructionRows=1000000
input.source.profiles[0][2].controlPoints[1][1]=2+Number.EPSILON*2
input.material.trims[0][2].controlPoints[1][1]=2+Number.EPSILON*2
const strict=request(input);assert.equal(strict.certified,false);assert.equal(strict.model,null)
input.limits.smoothness.order=0
const c0=request(input);assert.equal(c0.certified,true);assert.equal(c0.seamContinuity,'C0')
assert.equal(c0.fullBoundaryG1Certified,false);assert.equal(c0.fullBoundaryG2Certified,false)
assert.equal(c0.c0.allFacesRegular,true);assert.equal(c0.c0.edgeCorrespondenceCertified,true)
const report={schema:'original-rational-cap-wasm-runtime/1',wasmSha256:createHash('sha256').update(binary).digest('hex'),wasmBytes:binary.length,cases,sharpC0Passed:true,passed:true,scope:'Six specified original rational annular-cap WASM ABI fixtures; no public package, browser/UI or all-cap-law qualification'}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
