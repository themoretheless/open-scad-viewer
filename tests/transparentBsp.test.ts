import {expect,it} from 'vitest'
import {TransparentBsp, type TransparentFragment} from '../src/services/transparentBsp'
const red:TransparentFragment={owner:'red',triangle:[[-1,-1,-.6],[1,-1,.6],[0,1,0]]}
const blue:TransparentFragment={owner:'blue',triangle:[[-1,-1,.6],[1,-1,-.6],[0,1,0]]}
function hits(tree:TransparentBsp,x:number,direction:1|-1){return tree.ordered([0,0,direction]).filter(f=>{
 const [a,b,c]=f.triangle,y=-.5,det=(b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1]);
 const u=((b[1]-c[1])*(x-c[0])+(c[0]-b[0])*(y-c[1]))/det,v=((c[1]-a[1])*(x-c[0])+(a[0]-c[0])*(y-c[1]))/det
 return u>0&&v>0&&u+v<1
 }).map(f=>f.owner)}
it('orders crossing surfaces per ray and reverses with the camera',()=>{
 for(const input of [[red,blue],[blue,red]]){
  const tree=new TransparentBsp(input)
  expect(tree.fragmentCount).toBe(3)
  expect(hits(tree,-.25,1)).toEqual(['red','blue']);expect(hits(tree,.25,1)).toEqual(['blue','red'])
  expect(hits(tree,-.25,-1)).toEqual(['blue','red']);expect(hits(tree,.25,-1)).toEqual(['red','blue'])
 }
})
it('isolates returned attributes and refuses fragment overflow',()=>{
 const tree=new TransparentBsp([red,blue]);const first=tree.ordered([0,0,1]);(first[0].triangle[0] as number[])[0]=999
 expect(tree.ordered([0,0,1]).flatMap(f=>f.triangle).some(v=>v[0]===999)).toBe(false)
 expect(()=>new TransparentBsp([red,blue],2)).toThrow('fragment limit')
 expect(()=>tree.ordered([0,0,0])).toThrow('direction')
})

it('bounds work independently of fragment count on separated layers',()=>{
 const layers:TransparentFragment[]=Array.from({length:100},(_,i)=>({owner:String(i),triangle:[[-1,-1,i],[1,-1,i],[0,1,i]]}))
 const small=new TransparentBsp(layers.slice(0,3),1000,1e-9,100)
 expect(small.operationCount).toBe(5);expect(small.fragmentCount).toBe(3)
 expect(()=>new TransparentBsp(layers,1000,1e-9,100)).toThrow('operation limit')
 expect(()=>new TransparentBsp([],1000,1e-9,NaN)).toThrow('limits')
})

it('builds 1000 parallel layers within the existing work limit in both input orders',()=>{
 const layers:TransparentFragment[]=Array.from({length:1000},(_,i)=>({owner:String(i),triangle:[[-1,-1,i],[1,-1,i],[0,1,i]]}))
 for(const input of [layers,[...layers].reverse()]){
  const tree=new TransparentBsp(input)
  expect(tree.fragmentCount).toBe(1000);expect(tree.operationCount).toBeLessThan(11000)
  expect(tree.ordered([0,0,1]).map(f=>Number(f.owner))).toEqual(Array.from({length:1000},(_,i)=>i))
 }
})

it('writes the same attributes into reusable packed storage for changing cameras',()=>{
 const tree=new TransparentBsp([red,blue]),target=new Float32Array(tree.fragmentCount*9)
 for(const direction of [[0,0,1],[0,0,-1],[1,1,1]] as const){
  target.fill(NaN);tree.writeOrdered(direction,target)
  expect(target).toEqual(Float32Array.from(tree.ordered(direction).flatMap(f=>f.triangle.flatMap(v=>[...v]))))
 }
 target.fill(999);tree.writeOrdered([0,0,1],target);expect(target.includes(999)).toBe(false)
 expect(()=>tree.writeOrdered([0,0,1],new Float32Array(1))).toThrow('output size')
})

it('matches independent ray depths across spatially partitioned crossing planes, scales and input orders',()=>{
 const xy=[[-1,-1],[1,-1],[0,1]]
 for(const scale of [.001,1,1000]){
  const source:TransparentFragment[]=Array.from({length:24},(_,i)=>({owner:String(i),triangle:xy.map(([x,y])=>[x*scale,y*scale,(Math.sin(i*1.7)*x+Math.cos(i*2.3)*y+i*.017)*scale]) as unknown as TransparentFragment['triangle']}))
  const depth=(f:TransparentFragment,x:number,y:number)=>{
   const [a,b,c]=f.triangle,det=(b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1])
   const u=((b[1]-c[1])*(x-c[0])+(c[0]-b[0])*(y-c[1]))/det
   const v=((c[1]-a[1])*(x-c[0])+(a[0]-c[0])*(y-c[1]))/det
   return u>1e-10&&v>1e-10&&u+v<1-1e-10?u*a[2]+v*b[2]+(1-u-v)*c[2]:null
  }
  for(const input of [source,[...source].reverse()]){
   const tree=new TransparentBsp(input)
   for(const direction of [1,-1] as const){
    const ordered=tree.ordered([0,0,direction])
    for(let ix=0;ix<9;ix++)for(let iy=0;iy<7;iy++){
     const x=(-.61+ix*.137)*scale,y=(-.73+iy*.113)*scale
     const expected=source.map(f=>({owner:f.owner,z:depth(f,x,y)})).filter(p=>p.z!==null).sort((a,b)=>direction*(a.z!-b.z!)).map(p=>p.owner)
     const actual=ordered.filter(f=>depth(f,x,y)!==null).map(f=>f.owner)
     expect(actual,`scale=${scale}, view=${direction}, ray=${ix},${iy}`).toEqual(expected)
    }
   }
  }
 }
})

it('matches ray-triangle intersections for oblique camera directions',()=>{
 const source:TransparentFragment[]=Array.from({length:24},(_,i)=>({owner:String(i),triangle:[[-1,-1],[1,-1],[0,1]].map(([x,y])=>[x,y,Math.sin(i*1.7)*x+Math.cos(i*2.3)*y+i*.017]) as unknown as TransparentFragment['triangle']}))
 const cross=(a:number[],b:number[])=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
 const dot=(a:number[],b:number[])=>a.reduce((s,v,i)=>s+v*b[i],0)
 const sub=(a:readonly number[],b:readonly number[])=>a.map((v,i)=>v-b[i])
 for(const direction of [[.3,.4,1],[-.7,.2,1],[1,.4,-.3]]){
  const length=Math.hypot(...direction),d=direction.map(v=>v/length)
  const raw=cross(d,[0,1,0]),right=raw.map(v=>v/Math.hypot(...raw)),up=cross(right,d)
  const hit=(f:TransparentFragment,o:number[])=>{
   const [a,b,c]=f.triangle,e1=sub(b,a),e2=sub(c,a),p=cross(d,e2),det=dot(e1,p)
   if(Math.abs(det)<1e-12)return null
   const t=sub(o,a),u=dot(t,p)/det,q=cross(t,e1),v=dot(d,q)/det
   if(u<=1e-9||v<=1e-9||u+v>=1-1e-9)return null
   return dot(e2,q)/det
  }
  for(const input of [source,[...source].reverse()]){
   const ordered=new TransparentBsp(input).ordered(d as [number,number,number])
   let covered=0
   for(let ix=0;ix<11;ix++)for(let iy=0;iy<9;iy++){
    const o=right.map((v,i)=>v*(-.71+ix*.137)+up[i]*(-.63+iy*.143))
    const expected=source.map(f=>({id:f.owner,t:hit(f,o)})).filter(p=>p.t!==null).sort((a,b)=>a.t!-b.t!).map(p=>p.id)
    const actual=ordered.filter(f=>hit(f,o)!==null).map(f=>f.owner)
    if(expected.length>1)covered++
    expect(actual).toEqual(expected)
   }
   expect(covered).toBeGreaterThan(20)
  }
 }
})

it('preserves depth order of distinct near-coplanar layers',()=>{
 const layer=(owner:string,z:number):TransparentFragment=>({owner,triangle:[[-1,-1,z],[1,-1,z],[0,1,z]]})
 for(const gap of [1e-8,1e-10,1e-12])for(const input of [[layer('low',0),layer('high',gap)],[layer('high',gap),layer('low',0)]]){
  const tree=new TransparentBsp(input)
  expect(tree.ordered([0,0,1]).map(f=>f.owner)).toEqual(['low','high'])
  expect(tree.ordered([0,0,-1]).map(f=>f.owner)).toEqual(['high','low'])
 }
})

it('preserves per-ray crossing order across spatially separated groups',()=>{
 const source:TransparentFragment[]=Array.from({length:20},(_,i)=>[red,blue].map(f=>({owner:`${i}:${f.owner}`,triangle:f.triangle.map(v=>[v[0]+i*3,v[1],v[2]]) as unknown as TransparentFragment['triangle']}))).flat()
 for(const input of [source,[...source].reverse()]){
  const tree=new TransparentBsp(input)
  for(let i=0;i<20;i++)for(const direction of [1,-1] as const){
   const left=[`${i}:red`,`${i}:blue`],right=[...left].reverse()
   expect(hits(tree,i*3-.25,direction)).toEqual(direction===1?left:right)
   expect(hits(tree,i*3+.25,direction)).toEqual(direction===1?right:left)
  }
 }
})

it('does not subdivide a coplanar tessellation at spatial partition planes',()=>{
 const source:TransparentFragment[]=Array.from({length:64},(_,i)=>({owner:String(i),triangle:[[i,-1,0],[i+1,-1,0],[i,.5,0]]}))
 const tree=new TransparentBsp(source)
 expect(tree.fragmentCount).toBe(source.length)
 expect(tree.ordered([0,0,1])).toEqual(source)
})

it('preserves per-ray painter order when a world tree is projected through changing cameras',async()=>{
 const {createDirectProjector}=await import('../src/services/directModelingTools')
 const scene=[red,blue,{owner:'green',triangle:[[-1,-1,-1],[1,-1,-1],[0,1,1]]} as TransparentFragment],world=new TransparentBsp(scene)
 const owners=(fragments:TransparentFragment[],x:number,y:number)=>fragments.filter(({triangle:[a,b,c]})=>{
  const det=(b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1]);if(Math.abs(det)<1e-12)return false
  const u=((b[1]-c[1])*(x-c[0])+(c[0]-b[0])*(y-c[1]))/det,v=((c[1]-a[1])*(x-c[0])+(a[0]-c[0])*(y-c[1]))/det
  return u>1e-7&&v>1e-7&&u+v<1-1e-7
 }).map(f=>f.owner)
 let checked=0
 for(let i=0;i<25;i++){
  const camera={yaw:i*.41,pitch:-1.1+i*.09},project=createDirectProjector(camera),direction:[number,number,number]=[Math.sin(camera.yaw)*Math.cos(camera.pitch),Math.cos(camera.yaw)*Math.cos(camera.pitch),Math.sin(camera.pitch)]
  const projected=(items:TransparentFragment[])=>items.map(f=>({owner:f.owner,triangle:f.triangle.map(p=>project([...p])) as TransparentFragment['triangle']}))
  const actual=projected(world.ordered(direction)),expected=new TransparentBsp(projected(scene)).ordered([0,0,1])
  for(let x=-.91;x<1;x+=.137)for(let y=-.89;y<1;y+=.151){const a=owners(actual,x,y),b=owners(expected,x,y);expect(a).toEqual(b);if(a.length>1)checked++}
 }
 expect(checked).toBeGreaterThan(100)
})

it('matches the frozen BSP for separated layers and preserves crossing surfaces and attributes',async()=>{
 const {ReferenceTransparentBsp}=await import('../benchmarks/modelgraph/transparentBsp-reference')
 const groups=[[],[red,blue],Array.from({length:100},(_,i)=>({owner:String(i),triangle:[[-1,-1,i],[1,-1,i],[0,1,i]]} as TransparentFragment)),Array.from({length:24},(_,i)=>({owner:String(i),triangle:[[-1,-1],[1,-1],[0,1]].map(([x,y])=>[x,y,Math.sin(i*1.7)*x+Math.cos(i*2.3)*y+i*.017,x+2*y]) as unknown as TransparentFragment['triangle']}))]
 for(const source of groups){
  const before=structuredClone(source),actual=new TransparentBsp(source),expected=new ReferenceTransparentBsp(source)
  if(source.length!==24){expect(actual.operationCount).toBe(expected.operationCount);expect(actual.fragmentCount).toBe(expected.fragmentCount)}
  else {
   // Roundoff can change tiny sliver fragmentation; compare the represented surfaces.
   const areas=(tree:TransparentBsp|InstanceType<typeof ReferenceTransparentBsp>)=>{
    const totals=new Map<string,number>()
    for(const f of tree.ordered([0,0,1])){
     const [a,b,c]=f.triangle,u=b.map((v,k)=>v-a[k]),v=c.map((v,k)=>v-a[k])
     const area=Math.hypot(u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0])/2
     totals.set(f.owner,(totals.get(f.owner)??0)+area)
     for(const p of f.triangle)expect(p[3]).toBeCloseTo(p[0]+2*p[1],10)
    }
    return totals
   }
   const a=areas(actual),b=areas(expected)
   expect([...a.keys()].sort()).toEqual([...b.keys()].sort())
   for(const [id,area] of a)expect(area).toBeCloseTo(b.get(id)!,10)
   expect(actual.operationCount).toBeLessThanOrEqual(100000)
   continue
  }
  for(const direction of [[0,0,1],[0,0,-1],[.3,.4,1]] as const){
   const a=actual.ordered(direction),b=expected.ordered(direction)
   expect(a.map(f=>f.owner)).toEqual(b.map(f=>f.owner))
   a.forEach((f,i)=>f.triangle.forEach((v,j)=>v.forEach((x,k)=>expect(x).toBeCloseTo(b[i].triangle[j][k],10))))
  }
  expect(source).toEqual(before)
 }
})

it('handles empty and degenerate buffers and leaves failed builds reusable',()=>{
 const empty=new TransparentBsp([])
 expect(empty.fragmentCount).toBe(0);expect(empty.ordered([0,0,1])).toEqual([])
 empty.writeOrdered([0,0,1],new Float32Array())
 expect(new TransparentBsp([{owner:'zero',triangle:[[0,0,0],[0,0,0],[0,0,0]]}]).fragmentCount).toBe(0)
 expect(()=>new TransparentBsp([{owner:'bad',triangle:[[NaN,0,0],[1,0,0],[0,1,0]]}])).toThrow('vertex')
 expect(()=>new TransparentBsp([{owner:'bad',triangle:[[0,0,0],[1,0],[0,1,0]]}])).toThrow('width')
 expect(()=>new TransparentBsp([red,blue],2)).toThrow('fragment limit')
 expect(new TransparentBsp([red,blue]).fragmentCount).toBe(3)
})
