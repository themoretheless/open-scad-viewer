import {decimateLattice} from './latticeDecimation'
import {callGeometryRust} from './geometry/kernel'
import type {DirectBody} from './directModeling'
import {booleanPolygonMeshes,extrudePolygonProfile,inspectPolygonMesh} from './geometry/polygon'
export type LighteningPattern='bone'|'spatial'|'octet'|'bcc'|'isogrid'|'grid'|'triangles'|'honeycomb'|'web'
export interface LighteningOptions {pattern:LighteningPattern;axis:'x'|'y'|'z';cell:number;rib:number;rim:number;bottom:number;top:number;seed:number;jitter:number;lineWidth:number;perimeters:number;skin?:number;step?:number;openTop?:boolean;diagonals?:boolean;wallDepth?:number;keepCore?:boolean}
export type LatticeLoadClass='stretch'|'mixed'|'bending'|'organic'
type P=[number,number]
function clip(poly:P[],n:P,d:number):P[]{const result:P[]=[];for(let i=0;i<poly.length;i++){const a=poly[i],b=poly[(i+1)%poly.length],da=a[0]*n[0]+a[1]*n[1]-d,db=b[0]*n[0]+b[1]*n[1]-d;if(da<=1e-9)result.push(a);if((da<0)!==(db<0)){const t=da/(da-db);result.push([a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])])}}return result}
function inset(poly:P[],distance:number){let result=poly;for(let i=0;i<poly.length;i++){const a=poly[i],b=poly[(i+1)%poly.length],dx=b[0]-a[0],dy=b[1]-a[1],length=Math.hypot(dx,dy);if(length<1e-8)continue;const n:P=[dy/length,-dx/length];result=clip(result,n,a[0]*n[0]+a[1]*n[1]-distance)}return result}
const area=(p:P[])=>Math.abs(p.reduce((s,a,i)=>{const b=p[(i+1)%p.length];return s+a[0]*b[1]-a[1]*b[0]},0))/2
export function isSpatialPattern(pattern:LighteningPattern){return pattern==='bone'||pattern==='spatial'||pattern==='octet'||pattern==='bcc'}
/** Qualitative structural guidance — not a certified FEA result. */
export function latticeStructureHint(pattern:LighteningPattern):{rank:number;load:LatticeLoadClass;ru:string;en:string}{
 const table:Record<LighteningPattern,{rank:number;load:LatticeLoadClass;ru:string;en:string}>= {
  octet:{rank:1,load:'stretch',ru:'Октет-ферма: растянутый каркас с лучшим отношением жёсткость/масса в объёме и в скелетных стенках.',en:'Octet truss: stretch-dominated; best stiffness-to-weight among volumetric and skeletal-wall modes.'},
  isogrid:{rank:2,load:'stretch',ru:'Изогрид: равносторонние треугольные ячейки — классика облегчённых стенок (NASA isogrid).',en:'Isogrid: equilateral triangular cells — the classic lightened-wall layout (NASA isogrid).'},
  bcc:{rank:3,load:'mixed',ru:'ОЦК (BCC): стержни к центрам ячеек, хорошо держит сжатие при умеренном объёме.',en:'BCC: body-centered struts; strong in compression with moderate volume.'},
  triangles:{rank:4,load:'stretch',ru:'Диагональный каркас: треугольники повышают жёсткость относительно прямоугольной сетки.',en:'Diagonal skeleton: triangulation stiffens the wall versus a plain grid.'},
  honeycomb:{rank:5,load:'mixed',ru:'Соты: эффективны в плоскости панели, слабее на изгиб тонкой стенки.',en:'Honeycomb: efficient in-plane; weaker for thin-wall bending.'},
  spatial:{rank:6,load:'mixed',ru:'Прямоугольная 3D-сеть; с диагоналями ближе к растянутому каркасу.',en:'Axis-aligned 3D grid; diagonals move it toward stretch-dominated behaviour.'},
  bone:{rank:7,load:'organic',ru:'Кость: органическая пористость, не оптимизация по напряжениям.',en:'Bone: organic porosity, not stress-based topology optimization.'},
  web:{rank:8,load:'bending',ru:'Случайная паутина: снижает объём, жёсткость непредсказуема.',en:'Random web: reduces volume; stiffness is unpredictable.'},
  grid:{rank:9,load:'bending',ru:'Прямоугольная сетка: больше изгибных режимов, ниже удельная жёсткость.',en:'Rectangular grid: more bending modes and lower specific stiffness.'},
 }
 return table[pattern]
}
export function lighteningCells(min:P,max:P,o:LighteningOptions):P[][]{
 const nx=Math.max(1,Math.ceil((max[0]-min[0])/o.cell)),ny=Math.max(1,Math.ceil((max[1]-min[1])/o.cell));if(nx*ny>144)throw Error('More than 144 cells. Increase cell size.')
 const dx=(max[0]-min[0])/nx,dy=(max[1]-min[1])/ny,rect=(x:number,y:number,w:number,h:number):P[]=>[[x,y],[x+w,y],[x+w,y+h],[x,y+h]],boundary=rect(min[0],min[1],max[0]-min[0],max[1]-min[1]),cells:P[][]=[]
 if(o.pattern==='grid'||o.pattern==='triangles'){for(let y=0;y<ny;y++)for(let x=0;x<nx;x++){const p=rect(min[0]+x*dx,min[1]+y*dy,dx,dy);if(o.pattern==='grid')cells.push(p);else cells.push([p[0],p[1],p[2]],[p[0],p[2],p[3]])}}
 else if(o.pattern==='isogrid'){
  const s=o.cell,h=s*Math.sqrt(3)/2,cols=Math.ceil((max[0]-min[0])/s)+2,rows=Math.ceil((max[1]-min[1])/h)+2
  if(cols*rows*2>144)throw Error('More than 144 cells. Increase cell size.')
  const vx=(i:number,j:number):P=>[min[0]+(i+j*.5)*s,min[1]+j*h]
  const clipRect=(tri:P[])=>{let p=tri;p=clip(p,[1,0],max[0]);p=clip(p,[-1,0],-min[0]);p=clip(p,[0,1],max[1]);p=clip(p,[0,-1],-min[1]);if(p.length>=3&&area(p)>o.rib*o.rib/4)cells.push(p)}
  for(let j=-1;j<rows;j++)for(let i=-Math.floor(j/2)-2;i<cols;i++){clipRect([vx(i,j),vx(i+1,j),vx(i,j+1)]);clipRect([vx(i+1,j),vx(i+1,j+1),vx(i,j+1)])}
 }
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
 if(isSpatialPattern(o.pattern))return spatialLattice(body,o)
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

function bodyBounds(body:DirectBody){
 const min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity]
 for(let i=0;i<body.mesh.positions.length;i++){const k=i%3;min[k]=Math.min(min[k],body.mesh.positions[i]);max[k]=Math.max(max[k],body.mesh.positions[i])}
 return {min,max}
}
function edgeKey(a:number,b:number){return a<b?`${a},${b}`:`${b},${a}`}
function finishGraph(nodes:number[][],edgeSet:Set<string>){
 const edges=[...edgeSet].map(k=>k.split(',').map(Number))
 if(nodes.length>125||edges.length>400)throw Error('Spatial graph exceeds 125 nodes or 400 edges. Increase cell size.')
 if(!nodes.length||!edges.length)throw Error('Spatial graph is empty. Increase body size or reduce cell size.')
 return {nodes,edges}
}
function cubicCells(body:DirectBody,o:LighteningOptions){
 const {min,max}=bodyBounds(body),cells=max.map((v,k)=>Math.max(1,Math.ceil((v-min[k])/o.cell)))
 return {min,max,cells,span:max.map((v,k)=>v-min[k])}
}
function cornerGraph(body:DirectBody,o:LighteningOptions){
 const {min,max,cells}=cubicCells(body,o),counts=cells.map(n=>n+1);if(counts.reduce((a,b)=>a*b,1)>125)throw Error('Spatial graph exceeds 125 nodes. Increase cell size.')
 let seed=o.seed>>>0;const random=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296},nodes:number[][]=[],edges:number[][]=[],id=(x:number,y:number,z:number)=>(z*counts[1]+y)*counts[0]+x
 for(let z=0;z<counts[2];z++)for(let y=0;y<counts[1];y++)for(let x=0;x<counts[0];x++)nodes.push([x,y,z].map((v,k)=>min[k]+(v+(v>0&&v<cells[k]?(random()-.5)*o.jitter:0))*(max[k]-min[k])/cells[k]))
 for(let z=0;z<counts[2];z++)for(let y=0;y<counts[1];y++)for(let x=0;x<counts[0];x++){const a=id(x,y,z);if(x<cells[0])edges.push([a,id(x+1,y,z)]);if(y<cells[1])edges.push([a,id(x,y+1,z)]);if(z<cells[2])edges.push([a,id(x,y,z+1)]);if((o.diagonals||o.pattern==='bone')&&x<cells[0]&&y<cells[1]&&z<cells[2]){if(random()<.5)edges.push([a,id(x+1,y+1,z+1)]);else edges.push([id(x+1,y,z),id(x,y+1,z+1)])}}
 return {nodes,edges}
}
function bccGraph(body:DirectBody,o:LighteningOptions){
 const {min,cells,span}=cubicCells(body,o),cx=cells[0]+1,cy=cells[1]+1,cz=cells[2]+1,corner=(x:number,y:number,z:number)=>(z*cy+y)*cx+x
 const nodes:number[][]=[],edgeSet=new Set<string>()
 for(let z=0;z<cz;z++)for(let y=0;y<cy;y++)for(let x=0;x<cx;x++)nodes.push([min[0]+x*span[0]/cells[0],min[1]+y*span[1]/cells[1],min[2]+z*span[2]/cells[2]])
 const base=nodes.length
 for(let z=0;z<cells[2];z++)for(let y=0;y<cells[1];y++)for(let x=0;x<cells[0];x++){
  nodes.push([min[0]+(x+.5)*span[0]/cells[0],min[1]+(y+.5)*span[1]/cells[1],min[2]+(z+.5)*span[2]/cells[2]])
  const center=base+(z*cells[1]+y)*cells[0]+x
  for(const [dx,dy,dz] of [[0,0,0],[1,0,0],[0,1,0],[1,1,0],[0,0,1],[1,0,1],[0,1,1],[1,1,1]] as const)edgeSet.add(edgeKey(center,corner(x+dx,y+dy,z+dz)))
 }
 return finishGraph(nodes,edgeSet)
}
function octetGraph(body:DirectBody,o:LighteningOptions){
 const {min,cells,span}=cubicCells(body,o),cx=cells[0]+1,cy=cells[1]+1,cz=cells[2]+1,corner=(x:number,y:number,z:number)=>(z*cy+y)*cx+x
 const nodes:number[][]=[],edgeSet=new Set<string>(),at=(i:number,j:number,k:number,fx:number,fy:number,fz:number)=>[min[0]+(i+fx)*span[0]/cells[0],min[1]+(j+fy)*span[1]/cells[1],min[2]+(k+fz)*span[2]/cells[2]]
 for(let z=0;z<cz;z++)for(let y=0;y<cy;y++)for(let x=0;x<cx;x++)nodes.push(at(x,y,z,0,0,0))
 const xyBase=nodes.length
 for(let z=0;z<cz;z++)for(let y=0;y<cells[1];y++)for(let x=0;x<cells[0];x++)nodes.push(at(x,y,z,.5,.5,0))
 const xzBase=nodes.length
 for(let y=0;y<cy;y++)for(let z=0;z<cells[2];z++)for(let x=0;x<cells[0];x++)nodes.push(at(x,y,z,.5,0,.5))
 const yzBase=nodes.length
 for(let x=0;x<cx;x++)for(let z=0;z<cells[2];z++)for(let y=0;y<cells[1];y++)nodes.push(at(x,y,z,0,.5,.5))
 const xy=(x:number,y:number,z:number)=>xyBase+(z*cells[1]+y)*cells[0]+x
 const xz=(x:number,y:number,z:number)=>xzBase+(y*cells[2]+z)*cells[0]+x
 const yz=(x:number,y:number,z:number)=>yzBase+(x*cells[2]+z)*cells[1]+y
 const link=(a:number,b:number)=>{edgeSet.add(edgeKey(a,b))}
 for(let z=0;z<cz;z++)for(let y=0;y<cells[1];y++)for(let x=0;x<cells[0];x++){const f=xy(x,y,z);for(const [dx,dy] of [[0,0],[1,0],[0,1],[1,1]] as const)link(f,corner(x+dx,y+dy,z))}
 for(let y=0;y<cy;y++)for(let z=0;z<cells[2];z++)for(let x=0;x<cells[0];x++){const f=xz(x,y,z);for(const [dx,dz] of [[0,0],[1,0],[0,1],[1,1]] as const)link(f,corner(x+dx,y,z+dz))}
 for(let x=0;x<cx;x++)for(let z=0;z<cells[2];z++)for(let y=0;y<cells[1];y++){const f=yz(x,y,z);for(const [dy,dz] of [[0,0],[1,0],[0,1],[1,1]] as const)link(f,corner(x,y+dy,z+dz))}
 // Octahedron edges between face centres of each cubic cell.
 for(let z=0;z<cells[2];z++)for(let y=0;y<cells[1];y++)for(let x=0;x<cells[0];x++){
  const faces=[xy(x,y,z),xy(x,y,z+1),xz(x,y,z),xz(x,y+1,z),yz(x,y,z),yz(x+1,y,z)]
  for(const [a,b] of [[0,2],[0,3],[0,4],[0,5],[1,2],[1,3],[1,4],[1,5],[2,4],[2,5],[3,4],[3,5]] as const)link(faces[a],faces[b])
 }
 return finishGraph(nodes,edgeSet)
}
export function spatialGraph(body:DirectBody,o:LighteningOptions){
 if(o.pattern==='octet')return octetGraph(body,o)
 if(o.pattern==='bcc')return bccGraph(body,o)
 return cornerGraph(body,o)
}
function spatialLattice(body:DirectBody,o:LighteningOptions):DirectBody{
 const graph=spatialGraph(body,o),skin=o.skin??0,step=o.step??Math.min(o.rib/3,skin>0?skin/2:Infinity),mesh=callGeometryRust<import('./geometry/polygon').PolygonBuild>('mesh_spatial_lattice',{mesh:body.mesh,...graph,radius:o.rib/2,skin,step,organic:o.pattern==='bone',openTop:o.openTop??false,wallDepth:o.wallDepth??0,keepCore:o.keepCore??false})
 const reduced=decimateLattice(mesh,step*1.5),check=inspectPolygonMesh(reduced);if(!check.closed||check.degenerateTriangles||check.signedVolumeMm3<=0||check.signedVolumeMm3>=inspectPolygonMesh(body.mesh).signedVolumeMm3)throw Error('Lattice simplification failed topology checks. Increase grid resolution.');const result={...body,mesh:reduced};if(latticeComponents(result,true)>latticeComponents(body,true))throw Error('Clipping the spatial graph creates disconnected pieces. Increase strut thickness or add a skin.')
 return result
}
