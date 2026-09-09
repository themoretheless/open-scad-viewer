import {compileTextRust} from '../src/services/geometryRustKernel'
import {compileModelGraph} from '../src/services/modelGraph'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {readFileSync} from 'node:fs'
const g:any=compileTextRust(readFileSync('examples/skadis-box/skadis-dovetail.modelgraph.scad','utf8'))
for(const root of ['n83']){
 const reachable=new Set<string>();function visit(id:string){if(reachable.has(id))return;reachable.add(id);const n=g.nodes.find((n:any)=>n.id===id);for(const child of [n.input,n.base,...(n.inputs??[]),...(n.subtract??[]),n.then,n.else].filter(Boolean))visit(child)}visit(root)
 try{const c=compileModelGraph({language:'modelgraph/1',units:'mm',nodes:g.nodes.filter((n:any)=>reachable.has(n.id)),parameters:g.parameters,root,segments:40});const start=performance.now();let r=await parseOpenSCAD(c.source);for(const k of [12,72,264])console.log('triangle',k,Array.from({length:3},(_,j)=>Array.from(r.meshes[0].vertices.slice((k*3+j)*6,(k*3+j)*6+3))));console.log(root,Math.round(performance.now()-start),r.volume,r.meshes.map(m=>m.indices.length/3))}catch(e){console.log(root,String(e))}
}
