import {it,expect} from 'vitest'
import {lightenSolid,spatialGraph,latticeComponents,isSpatialPattern,type LighteningOptions} from '../src/services/solidLightening'
import {extrudeDirectSketch,directBodiesScad} from '../src/services/directModeling'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const box=()=>extrudeDirectSketch({id:'s',name:'Box',closed:true,points:[[0,0],[16,0],[16,12],[0,12]]},10,'0')
const o:LighteningOptions={pattern:'spatial',axis:'z',cell:10,rib:3.2,rim:0,bottom:0,top:0,seed:42,jitter:.5,lineWidth:.45,perimeters:3,skin:0,step:1.2,diagonals:true}
it.each(['bcc','octet'] as const)('generates bounded deterministic %s graphs and closed lighter solids',(pattern)=>{
 const b=box(),options={...o,pattern},g=spatialGraph(b,options)
 expect(isSpatialPattern(pattern)).toBe(true)
 expect(g.nodes).toHaveLength(pattern==='bcc'?22:38)
 expect(g.edges).toHaveLength(pattern==='bcc'?32:128)
 expect(new Set(g.edges.map(edge=>edge.join(','))).size).toBe(g.edges.length)
 expect(spatialGraph(b,{...options,seed:99,jitter:0,diagonals:false})).toEqual(g)
 const reached=new Set([0])
 for(let pass=0;pass<g.nodes.length;pass++)for(const [a,c] of g.edges){
  expect(a).toBeLessThan(c);expect(c).toBeLessThan(g.nodes.length)
  expect(g.nodes[a]).not.toEqual(g.nodes[c])
  if(reached.has(a)||reached.has(c)){reached.add(a);reached.add(c)}
 }
 expect(reached.size).toBe(g.nodes.length)
 for(const cell of [0,-1,Infinity,1e-300,2])expect(()=>spatialGraph(b,{...options,cell})).toThrow()
 const result=lightenSolid(b,options),report=inspectPolygonMesh(result.mesh)
 expect(report.closed).toBe(true);expect(report.degenerateTriangles).toBe(0)
 expect(report.signedVolumeMm3).toBeGreaterThan(0);expect(report.signedVolumeMm3).toBeLessThan(1920)
 expect(latticeComponents(result,true)).toBe(1)
},20000)
it('admits centered nodes and edges separately from corner count',()=>{
 const b=box(),scaled=(size:number[])=>({...b,mesh:{...b.mesh,positions:b.mesh.positions.map((v,i)=>v/[16,12,10][i%3]*size[i%3])}})
 expect(()=>spatialGraph(scaled([4,4,4]),{...o,pattern:'bcc',cell:1})).toThrow('125 nodes or 400 edges')
 expect(()=>spatialGraph(scaled([4,2,2]),{...o,pattern:'octet',cell:1})).toThrow('125 nodes or 400 edges')
 expect(spatialGraph(scaled([3,2,2]),{...o,pattern:'octet',cell:1}).edges).toHaveLength(352)
 expect(isSpatialPattern('grid')).toBe(false);expect(isSpatialPattern(undefined)).toBe(false)
})
it.each(['bcc','octet'] as const)('retains %s shell and wall-region options',(pattern)=>{
 const b=box(),options={...o,pattern,skin:2.4,wallDepth:3.6,keepCore:true}
 const closed=lightenSolid(b,{...options,openTop:false}),open=lightenSolid(b,{...options,openTop:true})
 for(const result of [closed,open]){
  const report=inspectPolygonMesh(result.mesh)
  expect(report.closed).toBe(true);expect(report.degenerateTriangles).toBe(0)
  expect(report.signedVolumeMm3).toBeGreaterThan(0);expect(report.signedVolumeMm3).toBeLessThan(1920)
 }
 expect(inspectPolygonMesh(open.mesh).signedVolumeMm3).toBeLessThan(inspectPolygonMesh(closed.mesh).signedVolumeMm3)
},20000)
it('has edges in all three dimensions and repeatable randomized nodes',()=>{const a=spatialGraph(box(),o),b=spatialGraph(box(),o);expect(a).toEqual(b);expect(a.nodes).not.toEqual(spatialGraph(box(),{...o,seed:43}).nodes);for(let k=0;k<3;k++)expect(a.edges.some(([i,j])=>a.nodes[i][k]!==a.nodes[j][k])).toBe(true)})
it.each(['spatial','bone'] as const)('generates a closed lighter %s volume',(pattern)=>{const b=box(),r=lightenSolid(b,{...o,pattern}),report=inspectPolygonMesh(r.mesh);expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeLessThan(1920);expect(report.signedVolumeMm3).toBeGreaterThan(0);expect(r.mesh.indices.length).toBeGreaterThan(100);expect(directBodiesScad({version:1,sketches:[],bodies:[r]}).length).toBeLessThan(250000)},20000)
it('rebuilds dense generated geometry from compact OpenSCAD source',async()=>{const {parseOpenSCAD}=await import('../src/services/openscadParser');const r=lightenSolid(box(),o),source=directBodiesScad({version:1,sketches:[],bodies:[r]});expect(source).toContain('polyhedron');let built;try{built=await parseOpenSCAD(source)}catch(e){throw Error(String(e).split('\n')[0]+' CODE '+source.slice((e as {start:number}).start-70,(e as {start:number}).start+70))}expect(built.meshes).toHaveLength(1)},20000)
it('retains an outer skin and supports opening its top',()=>{const b=box(),closed=lightenSolid(b,{...o,pattern:'bone',skin:2.4,openTop:false}),open=lightenSolid(b,{...o,pattern:'bone',skin:2.4,openTop:true});expect(inspectPolygonMesh(closed.mesh).closed).toBe(true);expect(inspectPolygonMesh(open.mesh).closed).toBe(true);expect(inspectPolygonMesh(open.mesh).signedVolumeMm3).toBeLessThan(inspectPolygonMesh(closed.mesh).signedVolumeMm3)},20000)
it('rejects unresolved ribs and excessive 3D node counts',()=>{expect(()=>lightenSolid(box(),{...o,step:3})).toThrow('step');expect(()=>spatialGraph(box(),{...o,cell:1})).toThrow('125')})
it('restricts lattice to walls with optional solid core',()=>{const walls=lightenSolid(box(),{...o,wallDepth:2.4,keepCore:false}),core=lightenSolid(box(),{...o,wallDepth:2.4,keepCore:true}),full=lightenSolid(box(),o);const v=(b:ReturnType<typeof box>)=>inspectPolygonMesh(b.mesh).signedVolumeMm3;expect(inspectPolygonMesh(walls.mesh).closed).toBe(true);expect(inspectPolygonMesh(core.mesh).closed).toBe(true);expect(v(walls)).toBeLessThan(v(full));expect(v(core)).toBeGreaterThan(v(walls));expect(v(core)).toBeLessThan(1920)},20000)
it('rejects wall layers below sampling resolution',()=>{expect(()=>lightenSolid(box(),{...o,wallDepth:1})).toThrow('Wall depth')})

it('preserves legacy graph fixtures, unsigned seeds and the exact node budget',()=>{
 const b=box(),g=spatialGraph(b,o)
 expect(g.nodes).toHaveLength(18);expect(g.edges).toHaveLength(37)
 ;[8.309124792926013,5.167662797961384,0].forEach((v,k)=>expect(g.nodes[4][k]).toBeCloseTo(v,12))
 ;[9.49525482300669,7.483902825973928,10].forEach((v,k)=>expect(g.nodes[13][k]).toBeCloseTo(v,12))
 for(const edge of [[1,12],[2,13],[4,15],[4,17]])expect(g.edges).toContainEqual(edge)
 expect(spatialGraph(b,{...o,seed:4294967338})).toEqual(g)
 const regular=spatialGraph(b,{...o,jitter:0,diagonals:false})
 expect(regular.edges).toHaveLength(33)
 for(const cell of [0,-1,Infinity,.001])expect(()=>spatialGraph(b,{...o,cell})).toThrow()
 for(const jitter of [-.1,1.1])expect(()=>spatialGraph(b,{...o,jitter})).toThrow()
 expect(()=>spatialGraph(b,{...o,seed:.5})).toThrow()
 const small={...b,mesh:{positions:b.mesh.positions.map((x,i)=>x/[16,12,10][i%3]*4),indices:b.mesh.indices}}
 expect(spatialGraph(small,{...o,cell:1} ).nodes).toHaveLength(125)
 expect(()=>spatialGraph(small,{...o,cell:.99})).toThrow('125')
})
it('counts indexed components and positive volumes independently of remote placement',()=>{
 const b=box(),n=b.mesh.positions.length/3
 const remote=b.mesh.positions.map((x,i)=>x+([1e12,-1e12,1e12][i%3]))
 const input={...b,mesh:{positions:[...b.mesh.positions,...remote],indices:[...b.mesh.indices,...b.mesh.indices.map(i=>i+n)]}},before=JSON.stringify(input)
 expect(latticeComponents(input)).toBe(2);expect(latticeComponents(input,true)).toBe(2)
 const reversed=[] as number[]
 for(let i=0;i<b.mesh.indices.length;i+=3)reversed.push(b.mesh.indices[i]+n,b.mesh.indices[i+2]+n,b.mesh.indices[i+1]+n)
 expect(latticeComponents({...input,mesh:{...input.mesh,indices:[...b.mesh.indices,...reversed]}},true)).toBe(1)
 expect(latticeComponents({...b,mesh:{positions:[],indices:[]}})).toBe(0)
 expect(latticeComponents({...b,mesh:{positions:[],indices:[]}},true)).toBe(0)
 expect(()=>latticeComponents({...b,mesh:{positions:[0,0,0],indices:[0,1,2]}})).toThrow()
 expect(JSON.stringify(input)).toBe(before)
})
