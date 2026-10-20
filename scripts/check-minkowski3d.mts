import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {box3d,lPrism3d,cavity3d,join3d,tetra3d} from '../tests/fixtures/minkowski3d-cases'
const bytes=readFileSync(process.argv[2]??'public/wasm/geometry-kernel.wasm'),w=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
const calls:any[]=[],proofs:any[]=[]
function call(op:string,args:object):any{const time=performance.now(),request=encodeBinary({op,...args}),p=writeLinear(w.memory,w.abi_alloc,request);assert(p);try{const r=decodePacked<any>(w.memory,w.abi_free,w.abi_request(0,p,request.length));assert(r.ok,JSON.stringify(r.error));return r.value}finally{w.abi_free(p,request.length);calls.push({op,milliseconds:performance.now()-time})}}
const kernel=box3d([-1,-1,-1],[1,1,1]),unit=box3d([0,0,0],[1,1,1]),slanted=box3d([0,0,0],[1,1,1]);for(let i=0;i<slanted.positions.length;i+=3)slanted.positions[i]+=slanted.positions[i+1]
const reversed=box3d([0,0,0],[1,1,1]);for(let i=0;i<reversed.indices.length;i+=3)[reversed.indices[i+1],reversed.indices[i+2]]=[reversed.indices[i+2],reversed.indices[i+1]]
const separated=join3d(unit,box3d([5,0,0],[6,1,1]))
for(const [name,a,b,volume] of [['boxes',box3d([0,0,0],[2,3,4]),kernel,120],['nonconvex',lPrism3d(),kernel,108],['cavity',cavity3d(),kernel,1720],['disconnected',separated,unit,16],['merged-components',separated,box3d([0,0,0],[5,1,1]),44],['slanted',box3d([0,0,0],[2,2,2]),slanted,33],['rational-display',tetra3d(),unit,17/3],['empty-positive-material',reversed,unit,0]] as const){
 const r=call('minkowski_3d_certify',{a,b});assert(Math.abs(r.volumeMm3-volume)<1e-8);assert.equal(r.status,'verified');assert.equal(r.curvedSourceErrorCertified,false)
 const verified=call('minkowski_3d_verify',{a,b,certificate:r.certificate});assert.deepEqual(verified.mesh,r.mesh)
 const forgeries:any[]=[];for(const mutation of ['source','boundary','census','schema']as const){const request={a:structuredClone(a),b,certificate:structuredClone(r.certificate)};if(mutation==='source')request.a.positions[0]+=.5;else if(mutation==='boundary'){if(request.certificate.triangles.length)request.certificate.triangles.pop();else request.certificate.vertices.push([['0','1'],['0','1'],['0','1']])}else if(mutation==='census')request.certificate.sumCells++;else request.certificate.untrusted=true
  assert.throws(()=>call('minkowski_3d_verify',request));forgeries.push({mutation,...request})}
 let meshReport=null;if(r.mesh.indices.length){meshReport=call('mesh_inspect',{mesh:r.mesh});assert(meshReport.closed);assert.equal(meshReport.orientationConflicts,0);assert.equal(meshReport.boundaryEdges,0);assert.equal(meshReport.nonManifoldEdges,0);assert.equal(meshReport.degenerateTriangles,0);assert(Math.abs(meshReport.signedVolumeMm3-volume)<1e-8)}
 if(name==='rational-display'){assert(r.maximumCoordinateRoundingMm>0);assert.equal(r.displayCoordinatesExact,false);assert.deepEqual(r.exactVolumeMm3,['17','3'])}
 proofs.push({name,a,b,certificate:r.certificate,verified:{status:r.status,scope:r.scope,volumeMm3:r.volumeMm3,exactVolumeMm3:r.exactVolumeMm3,maximumCoordinateRoundingMm:r.maximumCoordinateRoundingMm},meshReport,forgeries})
 console.error(name,'verified',r.volumeMm3)
}
const report={wasmSha256:createHash('sha256').update(bytes).digest('hex'),proofs,calls}
if(process.argv[3])writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n')
if(process.argv[4])writeFileSync(process.argv[4],JSON.stringify({version:1,sketches:[],bodies:[{id:'a',name:'Вогнутое тело',mesh:lPrism3d(),material:{name:'A',color:'#88aadd'}},{id:'b',name:'Ядро',mesh:kernel,material:{name:'B',color:'#cc9966'}}],curves:[],surfaces:[]},null,2)+'\n')
console.log(JSON.stringify({wasmSha256:report.wasmSha256,certificates:proofs.length,forgedRejected:proofs.reduce((n,p)=>n+p.forgeries.length,0),calls},null,2))
