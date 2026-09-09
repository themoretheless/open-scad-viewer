import type {PolygonMesh} from './polygonKernel'
import {cross3,dot3} from './directSketchGeometry'
type Edge={a:number;b:number;cost:number;va:number;vb:number}
/** Topology-preserving short-edge collapse. Cluster displacement is explicitly bounded. */
export function decimateLattice(mesh:PolygonMesh,tolerance:number,target=2600):PolygonMesh{
 const p=Array.from({length:mesh.positions.length/3},(_,i)=>mesh.positions.slice(i*3,i*3+3)),faces=Array.from({length:mesh.indices.length/3},(_,i)=>mesh.indices.slice(i*3,i*3+3)),alive=faces.map(()=>true),active=p.map(()=>true),version=p.map(()=>0),radius=p.map(()=>0),incident=p.map(()=>new Set<number>()),heap:Edge[]=[]
 faces.forEach((f,i)=>f.forEach(v=>incident[v].add(i)))
 const sub=(a:number[],b:number[])=>a.map((v,k)=>v-b[k]),distance=(a:number[],b:number[])=>Math.hypot(...sub(a,b))
 function push(a:number,b:number){if(a===b||!active[a]||!active[b])return;const e={a,b,cost:distance(p[a],p[b]),va:version[a],vb:version[b]};let i=heap.length;heap.push(e);while(i){const parent=(i-1)>>1;if(heap[parent].cost<=e.cost)break;heap[i]=heap[parent];i=parent}heap[i]=e}
 function pop(){const first=heap[0],last=heap.pop()!;if(heap.length){let i=0;while(i*2+1<heap.length){let child=i*2+1;if(child+1<heap.length&&heap[child+1].cost<heap[child].cost)child++;if(heap[child].cost>=last.cost)break;heap[i]=heap[child];i=child}heap[i]=last}return first}
 for(const f of faces)for(let i=0;i<3;i++)if(f[i]<f[(i+1)%3])push(f[i],f[(i+1)%3]);else push(f[(i+1)%3],f[i])
 let count=faces.length,attempts=0
 const neighbors=(v:number)=>new Set([...incident[v]].flatMap(i=>faces[i]).filter(i=>i!==v))
 while(count>target&&heap.length&&attempts++<1_000_000){const e=pop(),{a,b}=e;if(!active[a]||!active[b]||version[a]!==e.va||version[b]!==e.vb)continue
  const shared=[...incident[a]].filter(i=>incident[b].has(i));if(shared.length!==2)continue
  const na=neighbors(a),nb=neighbors(b),common=[...na].filter(i=>nb.has(i));if(common.length!==2)continue
  const point=p[a].map((v,k)=>(v+p[b][k])/2),r=Math.max(radius[a]+distance(point,p[a]),radius[b]+distance(point,p[b]));if(r>tolerance)continue
  const affected=new Set([...incident[a],...incident[b]]);let valid=true
  for(const i of affected){if(shared.includes(i))continue;const f=faces[i],old=f.map(v=>p[v]),next=f.map(v=>v===a||v===b?point:p[v]),n=cross3(sub(old[1],old[0]),sub(old[2],old[0])),m=cross3(sub(next[1],next[0]),sub(next[2],next[0]));if(dot3(m,m)<1e-16||dot3(n,m)<.2*Math.sqrt(dot3(n,n)*dot3(m,m))){valid=false;break}}
  if(!valid)continue
  const touched=new Set([...na,...nb,a,b]);for(const i of affected)for(const v of faces[i])incident[v].delete(i)
  p[a]=point;radius[a]=r;active[b]=false
  for(const i of affected){if(shared.includes(i)){alive[i]=false;count--;continue}faces[i]=faces[i].map(v=>v===b?a:v);for(const v of faces[i])incident[v].add(i)}
  for(const v of touched)version[v]++
  for(const v of touched)if(active[v])for(const n of neighbors(v))push(v,n)
 }
 const positions:number[]=[],indices:number[]=[],ids=new Map<number,number>();faces.forEach((f,i)=>{if(!alive[i])return;for(const v of f){let id=ids.get(v);if(id===undefined){id=positions.length/3;ids.set(v,id);positions.push(...p[v].map(v=>Math.round(v*1e6)/1e6))}indices.push(id)}});return {positions,indices}
}
