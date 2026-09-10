/** Fair warm SKADIS comparison: only the compiler changes; identical generated source
 * is parsed and built by the same uncached full-quality parseOpenSCAD call per sample.
 * Run: node --expose-gc --import tsx benchmarks/modelgraph/bench-modelgraph-end-to-end.mts
 */
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {cpus,totalmem,release} from 'node:os'
import {performance} from 'node:perf_hooks'
import {isDeepStrictEqual} from 'node:util'
import {execFileSync} from 'node:child_process'
import {compileModelGraphText as rust} from '../../src/services/modelGraphText'
import {compileModelGraphText as typescript} from './modelgraph-text-typescript-reference'
import {parseOpenSCAD,type ParseResult} from '../../src/services/openscadParser'

const output=process.argv.find(value=>value.startsWith('--output='))?.slice('--output='.length) ?? 'output/modelgraph-end-to-end-benchmark.json'
const input='examples/skadis-box/skadis-dovetail.modelgraph.scad'
const source=readFileSync(input,'utf8')
const sha=(data:Uint8Array|string)=>createHash('sha256').update(data).digest('hex')
const files=[input,'benchmarks/modelgraph/modelgraph-text-typescript-reference.ts','benchmarks/modelgraph/modelGraph-runtime-reference.ts','benchmarks/modelgraph/modelGraphTextNurbs-runtime-reference.ts','benchmarks/modelgraph/modelGraphNurbs-runtime-reference.ts','src/services/modelGraphText.ts','src/services/modelGraph.ts','src/services/openscadParser.ts','src/generated/geometry-kernels/kernel_bg.wasm','benchmarks/modelgraph/bench-modelgraph-end-to-end.mts']
const hashes=Object.fromEntries(files.map(path=>[path,sha(readFileSync(path))]))
const reference=typescript(source),actual=rust(source)
if(reference.source!==actual.source)throw new Error('Generated sources must be byte-identical for this comparison')
if(!isDeepStrictEqual(reference.document,actual.document))throw new Error('Canonical documents differ')
const generated=reference.source
const options={quality:'full' as const}
function summarize(scene:ParseResult){
 const bounds={min:[Infinity,Infinity,Infinity],max:[-Infinity,-Infinity,-Infinity]}
 for(const mesh of scene.meshes)for(let i=0;i<mesh.vertices.length;i+=6){
  const [x,y,z]=mesh.vertices.slice(i,i+3),t=mesh.transform
  const xyz=[t[0]*x+t[1]*y+t[2]*z+t[3],t[4]*x+t[5]*y+t[6]*z+t[7],t[8]*x+t[9]*y+t[10]*z+t[11]]
  for(let a=0;a<3;a++){bounds.min[a]=Math.min(bounds.min[a],xyz[a]);bounds.max[a]=Math.max(bounds.max[a],xyz[a])}
 }
 return {objects:scene.meshes.length,vertices:scene.meshes.reduce((sum,m)=>sum+m.vertices.length/6,0),triangles:scene.meshes.reduce((sum,m)=>sum+m.indices.length/3,0),volume:scene.volume,surface_area:scene.surfaceArea,bounds,warnings:scene.warnings,quality:scene.quality,reduced:scene.reduced,
  meshes:scene.meshes.map(mesh=>({vertices_sha256:sha(new Uint8Array(mesh.vertices.buffer,mesh.vertices.byteOffset,mesh.vertices.byteLength)),indices_sha256:sha(new Uint8Array(mesh.indices.buffer,mesh.indices.byteOffset,mesh.indices.byteLength)),topology:mesh.topology}))}
}
const compiler={typescript,rust}
const warmups:unknown[]=[]
for(let i=0;i<10;i++){typescript(source);rust(source)}
let expected:ReturnType<typeof summarize>|undefined
for(let round=0;round<2;round++)for(const name of (round%2?['rust','typescript']:['typescript','rust']) as (keyof typeof compiler)[]){
 const started=performance.now();const compiled=compiler[name](source);const scene=await parseOpenSCAD(compiled.source,options)
 const signature=summarize(scene);if(expected&&!isDeepStrictEqual(expected,signature))throw new Error('Warmup geometry differs');expected=signature
 warmups.push({round,compiler:name,total_ms:performance.now()-started,phases_ms:scene.timings})
}
const samples=[]
for(let round=0;round<5;round++)for(const name of (round%2?['rust','typescript']:['typescript','rust']) as (keyof typeof compiler)[]){
 // Collect out of the timing interval equally for both variants. No compiler,
 // geometry-result, AST or source cache is introduced by this harness.
 globalThis.gc?.()
 const started=performance.now();const compiled=compiler[name](source);const compiledAt=performance.now()
 const scene=await parseOpenSCAD(compiled.source,options);const builtAt=performance.now()
 if(compiled.source!==generated)throw new Error('Generated source changed during benchmark')
 if(!isDeepStrictEqual(expected,summarize(scene)))throw new Error('Measured geometry differs')
 const sample={round,compiler:name,compile_ms:compiledAt-started,parse_and_geometry_ms:builtAt-compiledAt,total_ms:builtAt-started,phases_ms:scene.timings}
 samples.push(sample);process.stdout.write(JSON.stringify(sample)+'\n')
}
const median=(values:number[])=>[...values].sort((a,b)=>a-b)[Math.floor(values.length/2)]
const medians=Object.fromEntries(Object.keys(compiler).map(name=>{
 const rows=samples.filter(sample=>sample.compiler===name)
 return [name,{compile_ms:median(rows.map(row=>row.compile_ms)),parse_and_geometry_ms:median(rows.map(row=>row.parse_and_geometry_ms)),total_ms:median(rows.map(row=>row.total_ms)),phases_ms:Object.fromEntries(['parseMs','initializeMs','evaluateMs','analyzeMs'].map(key=>[key,median(rows.map(row=>row.phases_ms[key as keyof typeof row.phases_ms]))]))}]
}))
const report={timestamp:new Date().toISOString(),environment:{node:process.version,platform:process.platform,architecture:process.arch,os_release:release(),cpu:cpus()[0]?.model,cpu_count:cpus().length,memory_bytes:totalmem(),explicit_gc:typeof globalThis.gc==='function',git_head:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),worktree_dirty:true},
 method:'Same source, identical canonical graph and byte-identical generated SCAD. Ten compiler warmups each, two alternating complete build warmups each, five alternating measured pairs. Every sample recompiles and calls parseOpenSCAD(generatedSource,{quality:full}); same geometry backend/options. GC requested before each timed sample equally. Geometry signatures verified after timer. Excludes browser worker transport/rendering and cold startup; compiler and geometry result caching not introduced.',
 hashes_before:hashes,hashes_after:Object.fromEntries(files.map(path=>[path,sha(readFileSync(path))])),generated_source_sha256:sha(generated),generated_source_characters:generated.length,scene:expected,warmups,samples,median_ms:medians}
writeFileSync(output,JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({output,median_ms:medians,scene:expected},null,2))
