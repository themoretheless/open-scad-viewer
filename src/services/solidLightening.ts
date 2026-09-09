import {decimateLattice} from './latticeDecimation'
import {callGeometryRust} from './geometryRustKernel'
import type {DirectBody} from './directModeling'
import {booleanPolygonMeshes,extrudePolygonProfile,inspectPolygonMesh} from './polygonKernel'
export type LighteningPattern='bone'|'spatial'|'grid'|'triangles'|'honeycomb'|'web'
export interface LighteningOptions {pattern:LighteningPattern;axis:'x'|'y'|'z';cell:number;rib:number;rim:number;bottom:number;top:number;seed:number;jitter:number;lineWidth:number;perimeters:number;skin?:number;step?:number;openTop?:boolean;diagonals?:boolean;wallDepth?:number;keepCore?:boolean}
type P=[number,number]
function clip(poly:P[],n:P,d:number):P[]{const result:P[]=[];for(let i=0;i<poly.length;i++){const a=poly[i],b=poly[(i+1)%poly.length],da=a[0]*n[0]+a[1]*n[1]-d,db=b[0]*n[0]+b[1]*n[1]-d;if(da<=1e-9)result.push(a);if((da<0)!==(db<0)){const t=da/(da-db);result.push([a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])])}}return result}
function inset(poly:P[],distance:number){let result=poly;for(let i=0;i<poly.length;i++){const a=poly[i],b=poly[(i+1)%poly.length],dx=b[0]-a[0],dy=b[1]-a[1],length=Math.hypot(dx,dy);if(length<1e-8)continue;const n:P=[dy/length,-dx/length];result=clip(result,n,a[0]*n[0]+a[1]*n[1]-distance)}return result}
const area=(p:P[])=>Math.abs(p.reduce((s,a,i)=>{const b=p[(i+1)%p.length];return s+a[0]*b[1]-a[1]*b[0]},0))/2
export function lighteningCells(min:P,max:P,o:LighteningOptions):P[][]{
 const nx=Math.max(1,Math.ceil((max[0]-min[0])/o.cell)),ny=Math.max(1,Math.ceil((max[1]-min[1])/o.cell));if(nx*ny>144)throw Error('More than 144 cells. Increase cell size.')
 const dx=(max[0]-min[0])/nx,dy=(max[1]-min[1])/ny,rect=(x:number,y:number,w:number,h:number):P[]=>[[x,y],[x+w,y],[x+w,y+h],[x,y+h]],boundary=rect(min[0],min[1],max[0]-min[0],max[1]-min[1]),cells:P[][]=[]
 if(o.pattern==='grid'||o.pattern==='triangles'){for(let y=0;y<ny;y++)for(let x=0;x<nx;x++){const p=rect(min[0]+x*dx,min[1]+y*dy,dx,dy);if(o.pattern==='grid')cells.push(p);else cells.push([p[0],p[1],p[2]],[p[0],p[2],p[3]])}}
 else if(o.pattern==='honeycomb'||o.pattern==='web'){
  let seed=o.seed>>>0;const random=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296},sites:P[]=[]
  // Hexagonal site lattice gives honeycomb cells; jitter gives reproducible irregular webs.
  const rows=Math.max(1,Math.ceil((max[1]-min[1])/(o.cell*Math.sqrt(3)/2)));if(nx*rows>144)throw Error('More than 144 sites. Increase cell size.')
  for(let y=0;y<rows;y++)for(let x=0;x<nx;x++){const jitter=o.pattern==='web'?o.jitter:0;sites.push([min[0]+(x+.25+(y%2)*.5+(random()-.5)*jitter)*dx,min[1]+(y+.5+(random()-.5)*jitter)*(max[1]-min[1])/rows])}
  for(const a of sites){let polygon=boundary;for(const b of sites){if(a===b)continue;const n:P=[b[0]-a[0],b[1]-a[1]],d=(b[0]*b[0]+b[1]*b[1]-a[0]*a[0]-a[1]*a[1])/2;polygon=clip(polygon,n,d);if(polygon.length<3)break}cells.push(polygon)}
 }else throw Error('Unknown lightening pattern.')
 return cells.map(p=>inset(p,o.rib/2)).filter(p=>p.length>=3&&area(p)>o.rib*o.rib/8)
}
export function latticeComponents(b:DirectBody,positiveOnly=false){
 const parent=Array.from({length:b.mesh.positions.length/3},(_,i)=>i),root=(i:number):number=>{while(parent[i]!==i){parent[i]=parent[parent[i]];i=parent[i]}return i}
 for(let i=0;i<b.mesh.indices.length;i+=3){const a=root(b.mesh.indices[i]);parent[root(b.mesh.indices[i+1])]=a;parent[root(b.mesh.indices[i+2])]=a}
 if(!positiveOnly)return new Set(b.mesh.indices.map(root)).size
 const volumes=new Map<number,number>(),origin=b.mesh.positions.slice(0,3)
 for(let i=0;i<b.mesh.indices.length;i+=3){const f=b.mesh.indices.slice(i,i+3),p=f.map(v=>b.mesh.positions.slice(v*3,v*3+3).map((x,k)=>x-origin[k])),v=(p[0][0]*(p[1][1]*p[2][2]-p[1][2]*p[2][1])+p[0][1]*(p[1][2]*p[2][0]-p[1][0]*p[2][2])+p[0][2]*(p[1][0]*p[2][1]-p[1][1]*p[2][0]))/6,r=root(f[0]);volumes.set(r,(volumes.get(r)??0)+v)}
 return [...volumes.values()].filter(v=>v>1e-8).length
}

export function lightenSolid(body:DirectBody,o:LighteningOptions):DirectBody{
 if(![o.cell,o.rib,o.rim,o.bottom,o.top,o.seed,o.jitter,o.lineWidth,o.perimeters].every(Number.isFinite)||o.cell<=0||o.rib<=0||o.rib>=o.cell/2||o.rim<0||o.bottom<0||o.top<0||o.jitter<0||o.jitter>1||!Number.isInteger(o.seed)||o.lineWidth<=0||!Number.isInteger(o.perimeters)||o.perimeters<1||o.perimeters>8)throw Error('Check cell size, rib width, borders, seed and print settings. Rib must be smaller than half a cell.')
 if(o.rib+1e-6<o.lineWidth*o.perimeters)throw Error('Rib is thinner than the requested number of extrusion lines. Increase rib width or change the print settings.')
 if(o.pattern==='bone'||o.pattern==='spatial')return spatialLattice(body,o)
 const axis=['x','y','z'].indexOf(o.axis);if(axis<0)throw Error('Choose X, Y or Z channel direction.');const u=(axis+1)%3,v=(axis+2)%3,min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];for(let i=0;i<body.mesh.positions.length;i++){const k=i%3;min[k]=Math.min(min[k],body.mesh.positions[i]);max[k]=Math.max(max[k],body.mesh.positions[i])}
 if(max[axis]-min[axis]<=o.bottom+o.top||max[u]-min[u]<=2*o.rim+o.rib||max[v]-min[v]<=2*o.rim+o.rib)throw Error('Skins or frame consume the available body.')
 const before=inspectPolygonMesh(body.mesh);if(!before.closed||before.signedVolumeMm3<=0)throw Error('Select a closed outward-oriented solid.')
 const cells=lighteningCells([min[u]+o.rim,min[v]+o.rim],[max[u]-o.rim,max[v]-o.rim],o);if(!cells.length)throw Error('No openings fit. Reduce rib or frame width.')
 const extension=Math.max(...max.map((x,i)=>x-min[i]))*1e-5+1e-5,start=min[axis]+o.bottom-(o.bottom===0?extension:0),end=max[axis]-o.top+(o.top===0?extension:0)
 const cutters={positions:[] as number[],indices:[] as number[]}
 for(const cell of cells){const cutter=extrudePolygonProfile({outer:cell},[0,0,end-start]),offset=cutters.positions.length/3;for(let i=0;i<cutter.positions.length;i+=3){const p=[0,0,0];p[u]=cutter.positions[i];p[v]=cutter.positions[i+1];p[axis]=cutter.positions[i+2]+start;cutters.positions.push(...p)}cutters.indices.push(...cutter.indices.map(i=>i+offset))}
 const mesh=booleanPolygonMeshes(body.mesh,cutters,'difference');if(!mesh.indices.length)throw Error('Lightening removed the entire body.')
 const report=inspectPolygonMesh(mesh),result={...body,mesh};if(!report.closed||report.signedVolumeMm3<=0)throw Error('Lightening did not produce a closed solid.');if(report.signedVolumeMm3>=before.signedVolumeMm3-1e-6)throw Error('No material was removed. Change channel direction or cell size.');if(latticeComponents(result)>latticeComponents(body))throw Error('Pattern creates disconnected pieces. Increase the frame/rib width or keep a bottom skin.')
 // Compact source coordinates while retaining the validated topology.
 const compact={...result,mesh:{positions:mesh.positions.map(v=>Math.round(v*1e6)/1e6),indices:mesh.indices}};const check=inspectPolygonMesh(compact.mesh);if(!check.closed||check.degenerateTriangles)throw Error('Pattern creates details below export precision.');return compact
}

export function spatialGraph(body:DirectBody,o:LighteningOptions){
 const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];for(let i=0;i<body.mesh.positions.length;i++){const k=i%3;min[k]=Math.min(min[k],body.mesh.positions[i]);max[k]=Math.max(max[k],body.mesh.positions[i])}
 const cells=max.map((v,k)=>Math.max(1,Math.ceil((v-min[k])/o.cell))),counts=cells.map(n=>n+1);if(counts.reduce((a,b)=>a*b,1)>125)throw Error('Spatial graph exceeds 125 nodes. Increase cell size.')
 let seed=o.seed>>>0;const random=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296},nodes:number[][]=[],edges:number[][]=[],id=(x:number,y:number,z:number)=>(z*counts[1]+y)*counts[0]+x
 for(let z=0;z<counts[2];z++)for(let y=0;y<counts[1];y++)for(let x=0;x<counts[0];x++)nodes.push([x,y,z].map((v,k)=>min[k]+(v+(v>0&&v<cells[k]?(random()-.5)*o.jitter:0))*(max[k]-min[k])/cells[k]))
 for(let z=0;z<counts[2];z++)for(let y=0;y<counts[1];y++)for(let x=0;x<counts[0];x++){const a=id(x,y,z);if(x<cells[0])edges.push([a,id(x+1,y,z)]);if(y<cells[1])edges.push([a,id(x,y+1,z)]);if(z<cells[2])edges.push([a,id(x,y,z+1)]);if((o.diagonals||o.pattern==='bone')&&x<cells[0]&&y<cells[1]&&z<cells[2]){if(random()<.5)edges.push([a,id(x+1,y+1,z+1)]);else edges.push([id(x+1,y,z),id(x,y+1,z+1)])}}
 return {nodes,edges}
}
function spatialLattice(body:DirectBody,o:LighteningOptions):DirectBody{
 const graph=spatialGraph(body,o),skin=o.skin??0,step=o.step??Math.min(o.rib/3,skin>0?skin/2:Infinity),mesh=callGeometryRust<import('./polygonKernel').PolygonBuild>('mesh_spatial_lattice',{mesh:body.mesh,...graph,radius:o.rib/2,skin,step,organic:o.pattern==='bone',openTop:o.openTop??false,wallDepth:o.wallDepth??0,keepCore:o.keepCore??false})
 const reduced=decimateLattice(mesh,step*1.5),check=inspectPolygonMesh(reduced);if(!check.closed||check.degenerateTriangles||check.signedVolumeMm3<=0||check.signedVolumeMm3>=inspectPolygonMesh(body.mesh).signedVolumeMm3)throw Error('Lattice simplification failed topology checks. Increase grid resolution.');const result={...body,mesh:reduced};if(latticeComponents(result,true)>latticeComponents(body,true))throw Error('Clipping the spatial graph creates disconnected pieces. Increase strut thickness or add a skin.')
 return result
}
