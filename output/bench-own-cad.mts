import {readFileSync,writeFileSync} from 'node:fs'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {parseOpenSCAD} from '../src/services/openscadParser'
const source=readFileSync('examples/skadis-box/skadis-dovetail.modelgraph.scad','utf8')
const samples=[]
let result
for(let i=0;i<6;i++){const start=performance.now();const compiled=compileModelGraphText(source);const compiledAt=performance.now();result=await parseOpenSCAD(compiled.source);samples.push({compileMs:compiledAt-start,geometryMs:performance.now()-compiledAt,totalMs:performance.now()-start})}
const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity]
for(const mesh of result!.meshes)for(let i=0;i<mesh.vertices.length;i+=6)for(let k=0;k<3;k++){min[k]=Math.min(min[k],mesh.vertices[i+k]);max[k]=Math.max(max[k],mesh.vertices[i+k])}
const report={engine:'own-rust-cad-v1',node:process.version,samples,cold:samples[0],warmMedianMs:samples.slice(1).map(s=>s.totalMs).sort((a,b)=>a-b)[2],volume:result!.volume,bounds:{min,max},triangles:result!.meshes.map(m=>m.indices.length/3),topology:result!.meshes.map(m=>m.topology)}
writeFileSync('output/own-cad-box-benchmark.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report))
