// Qualification transport only. Reconstruction and every proof run in Rust WASM.
import assert from 'node:assert/strict'
import {checkCompleteFacePairCover} from './sweep-face-pair-cover.mjs'
import {readFileSync, writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve} from 'node:path'
import {build} from 'esbuild'

const [wasmFile, captureFile, outputFile, stageDirectory] = process.argv.slice(2)
assert(wasmFile && captureFile && outputFile && stageDirectory,
  'Supply raw WASM, original Rust-captured FixedNormal request, output receipt and frozen stage')
const sha = bytes => createHash('sha256').update(bytes).digest('hex')
const codec = resolve(stageDirectory, 'src/services/valueBinaryCodec.ts')
const bundled = await build({entryPoints: [codec],
  bundle: true, write: false, platform: 'node', format: 'esm'})
const {encodeBinary, decodeBinary} = await import('data:text/javascript;base64,' + Buffer.from(bundled.outputFiles[0].contents).toString('base64'))
const bytes = readFileSync(wasmFile), baselineBytes = readFileSync(captureFile)
const captured = JSON.parse(baselineBytes)
const baseline=captured.cases.find(c=>!c.joint&&c.hollow);assert(baseline);
const source=structuredClone(baseline.source);
assert.equal(source.orientation, "fixed_normal")
assert.equal(source.spacing, "arc_length")
assert.equal(source.path.periodic, true)
assert.equal(source.loops.length, 2)
const {exports: wasm} = new WebAssembly.Instance(new WebAssembly.Module(bytes))
function request(value) {
  const encoded = encodeBinary(value), ptr = wasm.abi_alloc(encoded.length)
  assert(ptr, 'WASM request allocation failed')
  new Uint8Array(wasm.memory.buffer, ptr, encoded.length).set(encoded)
  try {
    const packed = wasm.abi_request(0, ptr, encoded.length)
    const responsePtr = Number(packed & 0xffffffffn), size = Number(packed >> 32n)
    let envelope
    try { envelope = decodeBinary(new Uint8Array(wasm.memory.buffer, responsePtr, size)) }
    finally { wasm.abi_free(responsePtr, size) }
    assert.equal(envelope.ok, true, JSON.stringify(envelope.error))
    return envelope.value
  } finally { wasm.abi_free(ptr, encoded.length) }
}

const started=performance.now();const retained=request(source).stationReconstruction;
const model=retained.reconstruction.model;
assert.equal(model.faces.length,256);assert.equal(model.shells.length,2);
assert.equal(retained.boundaryCertificate.continuousBound,true);
assert.equal(retained.boundaryCertificate.withinBudget,true);
assert.equal(retained.profileSmoothness.station.stationG2Certified,true);
const constructionMs=performance.now()-started;
const budgets={model,capFaces:[],toleranceUv:1e-8,maxExactWork:1000000,maxTrimPairs:10000,maxTrimCells:100000,maxTrimDomainCells:1000000,
 maxSpans:1024,maxLinearCells:100000,maxPairs:20000,maxCells:200000,maxDomainCells:1000000,cellsPerPair:1000,domainCellsPerPair:10000,
 nestingPairs:1000,nestingCells:100000,nestingDomainCells:1000000,orientationCells:100000,orientationDomainCells:1000000,orientationSpans:100,
 capBudgets:{maxWalls:1024,maxExactWork:1000000,maxChartCells:1000,maxTrimPairs:100000,maxTrimCells:100000,maxTrimDomainCells:1000000}};
const beforeBegin=performance.now();const begin=request({op:'brep_sweep_volume_begin',...budgets});const beginMs=performance.now()-beforeBegin;
let maxUnits=1,steps=0,largest=0,previous=0;const samples=[];
for(;;){
 const before=performance.now();const step=request({op:'brep_sweep_volume_step',session:begin.session,maxUnits});
 const elapsed=performance.now()-before;largest=Math.max(largest,elapsed);steps++;
 assert(step.completedSteps>=previous);previous=step.completedSteps;
 samples.push({step:steps,elapsedMs:elapsed,maxUnits,advancedUnits:step.advancedUnits,completedSteps:step.completedSteps});
 if(step.done){
  assert.deepEqual(step.report,baseline.volume);
  const pairCover=checkCompleteFacePairCover(step.report.boundary,model.faces.length);
  const result={completed:true,sha256:sha(bytes),baselineSha256:sha(baselineBytes),source,constructionMs,beginMs,totalMs:performance.now()-started,maxStepMs:largest,steps,pairCover,volume:step.report,samples};
  writeFileSync(outputFile,JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({completed:true,constructionMs,beginMs,totalMs:result.totalMs,maxStepMs:largest,steps}));break;
 }
 assert(!('report' in step));maxUnits=elapsed<8?Math.min(64,maxUnits*2):elapsed>50?Math.max(1,Math.floor(maxUnits/2)):maxUnits;
 if(steps%50===0){writeFileSync(outputFile,JSON.stringify({completed:false,constructionMs,beginMs,totalMs:performance.now()-started,maxStepMs:largest,steps,samples},null,2)+'\n');console.log(JSON.stringify({steps,elapsedMs:performance.now()-started,maxStepMs:largest}));}
 await new Promise(resolve=>setTimeout(resolve,0));
}
assert.equal(request({op:'brep_sweep_volume_drop',session:begin.session}).dropped,false);

const replayStart=performance.now();
const replay=request({op:'brep_sweep_volume_audit',...budgets,model:JSON.parse(JSON.stringify(model))});
const replayMs=performance.now()-replayStart;
const expected=structuredClone(baseline.volume);delete expected.retainedWallCharts;
assert.deepEqual(replay,expected);
const final=JSON.parse(readFileSync(outputFile,'utf8'));
final.snapshotAdmissionReplayMs=replayMs;final.snapshotAdmissionEqualsPrior=true;
writeFileSync(outputFile,JSON.stringify(final,null,2)+'\n');console.log(JSON.stringify({snapshotAdmissionReplayMs:replayMs}));
