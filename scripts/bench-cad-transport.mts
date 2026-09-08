import {mkdirSync,writeFileSync} from 'node:fs'
import {deepStrictEqual} from 'node:assert'
import {callGeometryRust,withCadMesh} from '../src/services/geometryRustKernel'
mkdirSync('output',{recursive:true})
const id=callGeometryRust<number>('cad',{action:'sphere',radius:10,segments:128})
const values=()=>callGeometryRust<{positions:number[];indices:number[];faceIds:number[]}>('cad',{action:'export_mesh',id})
try{
 const reference=values()
 withCadMesh(id,m=>{deepStrictEqual(Array.from(m.positions),reference.positions);deepStrictEqual(Array.from(m.indices),reference.indices);deepStrictEqual(Array.from(m.faceIds),reference.faceIds)})
 const valueMs:number[]=[],binaryMs:number[]=[];let checksum=0
 const measure=(binary:boolean)=>{const start=performance.now();if(binary)checksum+=withCadMesh(id,m=>m.positions.length+m.indices.length+m.faceIds.length);else{const m=values();checksum+=m.positions.length+m.indices.length+m.faceIds.length}return performance.now()-start}
 for(let i=0;i<35;i++){const binaryFirst=i%2===0;const first=measure(binaryFirst),second=measure(!binaryFirst);if(i>=5){binaryMs.push(binaryFirst?first:second);valueMs.push(binaryFirst?second:first)}}
 const median=(values:number[])=>[...values].sort((a,b)=>a-b)[Math.floor(values.length/2)]!
 const report={node:process.version,architecture:process.arch,triangles:reference.indices.length/3,iterations:30,valueMs,binaryMs,valueMedianMs:median(valueMs),binaryMedianMs:median(binaryMs),speedup:median(valueMs)/median(binaryMs),checksum,scope:'Binary value encoding versus direct mesh views; excludes CSG and renderer buffer construction'}
 writeFileSync('output/cad-binary-transport-benchmark.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({valueMedianMs:report.valueMedianMs,binaryMedianMs:report.binaryMedianMs,speedup:report.speedup,triangles:report.triangles}))
}finally{callGeometryRust('cad',{action:'delete',ids:[id]})}
