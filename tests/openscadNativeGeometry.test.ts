import {expect,it} from 'vitest'
import {evaluateNativeOpenScadGeometry} from '../src/services/openscadNativeGeometry'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {callGeometryRust} from '../src/services/geometry/kernel'
it('executes evaluated variables, user modules and loops entirely through the Rust plan',()=>{
 const result=evaluateNativeOpenScadGeometry('w=2;module part(x){translate([x,0,0])cube([w,3,4],center=true);}for(x=[1,5])part(x);')
 expect(result.ok).toBe(true);if(!result.ok)throw Error('Native evaluation failed')
 expect(result.meshes).toHaveLength(2)
 for(const mesh of result.meshes)expect(inspectPolygonMesh(mesh).signedVolumeMm3).toBeCloseTo(24,8)
 expect(Math.min(...Array.from(result.meshes[0].positions).filter((_,i)=>i%3===0))).toBe(0)
 expect(Math.min(...Array.from(result.meshes[1].positions).filter((_,i)=>i%3===0))).toBe(4)
})
it('matches the current evaluator volumes for primitive and boolean scenes',async()=>{
 for(const source of ['cube([2,3,4],true);','union(){cube(2);translate([1,0,0])cube(2);}','difference(){cube(3);translate([1,1,-1])cube([1,1,5]);}','intersection(){cube(3);translate([2,0,0])cube(3);}','cylinder(h=4,r=2,$fn=8,center=true);']){
  const native=evaluateNativeOpenScadGeometry(source),reference=await parseOpenSCAD(source)
  expect(native.ok).toBe(true);if(!native.ok)throw Error('Native evaluation failed')
  const volume=native.meshes.reduce((sum,m)=>sum+inspectPolygonMesh(m).signedVolumeMm3,0)
  expect(volume).toBeCloseTo(reference.volume,5)
 }
})
it('preserves preview warnings and refuses unsupported geometry explicitly',()=>{
 const result=evaluateNativeOpenScadGeometry('sphere(2,$fn=1000);',{quality:'preview'})
 expect(result.ok).toBe(true);if(result.ok){expect(result.reduced).toBe(true);expect(result.warnings).toContain('$fn=1000 was clamped to 48 for preview rendering')}
 for(const source of ['linear_extrude(2)square(1);','hull(){cube(1);cube(2);}','color("red")cube(1);']){
  const rejected=evaluateNativeOpenScadGeometry(source)
  expect(rejected.ok).toBe(false);if(!rejected.ok)expect(rejected.diagnostics?.[0].message).toContain('does not yet support')
 }
})
it('rejects cyclic and forward references before execution',()=>{
 expect(()=>callGeometryRust('solid_program_execute',{program:{nodes:[{kind:'translate',input:0,delta:[0,0,0]}],roots:[0]}})).toThrow('references')
})

it('refuses stable recording modules whose normalization is not connected',()=>{
 const result=evaluateNativeOpenScadGeometry('surface(file="missing.dat");',{profile:'openscad/stable-2021.01'})
 expect(result.ok).toBe(false)
 if(!result.ok)expect(result.diagnostics?.[0].message).toContain('does not yet support')
})

it('matches transform order, reflection winding and matrix layout with the current evaluator',async()=>{
 for(const transform of ['rotate([20,30,40])','rotate(90,[1,2,3])','rotate(90)','scale([-2,3,4])','mirror([1,2,3])','multmatrix([[1,.2,0,4],[0,2,.3,-3],[0,0,1,2]])','rotate([20,30,40])translate([1,2,3])scale([-2,3,4])']){
  const source=transform+'cube([2,3,4],center=true);',native=evaluateNativeOpenScadGeometry(source),reference=await parseOpenSCAD(source)
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native transform failed')
  const points=Array.from(native.meshes[0].positions),display=Array.from(reference.meshes[0].vertices)
  for(let axis=0;axis<3;axis++){
   const a=points.filter((_,i)=>i%3===axis),b=display.filter((_,i)=>i%6===axis)
   expect(Math.min(...a)).toBeCloseTo(Math.min(...b),5);expect(Math.max(...a)).toBeCloseTo(Math.max(...b),5)
  }
  expect(inspectPolygonMesh(native.meshes[0]).signedVolumeMm3).toBeCloseTo(reference.volume,5)
 }
})

it('executes stable cubes with shared defaults, lexical bindings and implicit union',async()=>{
 for(const source of ['cube([2,3,4],true);','cube([2,"bad",4]);','cube(2,center=1);','union(){cube(0);cube([2,3,4]);}','w=2;module part(){cube([w,3,4]);}part();','cube(2);cube(1);']){
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'}),reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Stable cube evaluation failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
 }
 const empty=evaluateNativeOpenScadGeometry('cube(0);',{profile:'openscad/stable-2021.01'})
 expect(empty.ok).toBe(true);if(empty.ok)expect(empty.meshes).toHaveLength(0)
 const partial=evaluateNativeOpenScadGeometry('cube([2,"bad",4]);',{profile:'openscad/stable-2021.01'})
 if(partial.ok)expect(partial.warnings).toContain('cube size was not a scalar or exact 3-component numeric vector; unit size is used')
})

it('preserves empty operands and their order in stable booleans',()=>{
 for(const [operation,first,second,volume] of [
  ['union',0,1,1],['union',1,0,1],['intersection',0,1,0],['intersection',1,0,0],['difference',0,1,0],['difference',1,0,1],
 ] as const){
  const source=`${operation}(){cube(${first});cube(${second});}`
  const result=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  expect(result.ok,source).toBe(true);if(!result.ok)throw Error('Stable boolean evaluation failed')
  expect(result.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(volume,8)
  if(volume===0)expect(result.meshes).toHaveLength(0)
 }
})

it('preserves explicit empty stable groups as boolean operands',()=>{
 for(const source of ['difference(){union(){}cube(1);}','intersection(){cube(1);union(){}}']){
  const result=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  expect(result.ok).toBe(true);if(result.ok)expect(result.meshes).toHaveLength(0)
 }
})

it('executes stable cylinders with radius precedence and fragment scopes',async()=>{
 for(const source of ['cylinder(h=2,r=1,$fn=8);','cylinder(2,0,1,true,$fn=3);','cylinder(h=2,r=9,d=2,r2=.5,$fn=8);','$fn=8;cylinder(h=3,r=2);','cylinder(h=2,r=2,$fa=45,$fs=100);']){
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'}),reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Stable cylinder evaluation failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
 }
 const empty=evaluateNativeOpenScadGeometry('cylinder(h=0,r=2);',{profile:'openscad/stable-2021.01'})
 expect(empty.ok).toBe(true);if(empty.ok)expect(empty.meshes).toHaveLength(0)
 const preview=evaluateNativeOpenScadGeometry('cylinder(h=2,r=1,$fn=1000);',{profile:'openscad/stable-2021.01',quality:'preview'})
 expect(preview.ok).toBe(true);if(preview.ok){expect(preview.reduced).toBe(true);expect(preview.warnings).toContain('Fragment count was clamped to the engine safety limit 48')}
})

it('executes stable vector transforms through the shared Rust plans',async()=>{
 for(const transform of ['translate([1,2,3])','translate([2,"bad",3])','scale([-2,3,4])','scale([2,"bad",4])','mirror([1,2,3])','mirror([0,0,0])','translate([1,2,3])scale(-2)mirror([1,0,0])','rotate([20,30,40])','rotate(45,[1,2,3])','rotate([2,"bad",4])','rotate(30,[0,0,0])','multmatrix([[2,0,0,3],[0,3,0,4],[0,0,-1,5]])','multmatrix([[2,0,0,4],[0,2,0,6],[0,0,2,8],[0,0,0,2]])','multmatrix([[2,"bad"],[0,3]])']){
  const source=transform+'cube([2,3,4],true);'
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'}),reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Stable vector transform failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
  const a=Array.from(native.meshes[0].positions),b=Array.from(reference.meshes[0].vertices)
  for(let axis=0;axis<3;axis++){
   expect(Math.min(...a.filter((_,i)=>i%3===axis))).toBeCloseTo(Math.min(...b.filter((_,i)=>i%6===axis)),5)
   expect(Math.max(...a.filter((_,i)=>i%3===axis))).toBeCloseTo(Math.max(...b.filter((_,i)=>i%6===axis)),5)
  }
 }
 const dropped=evaluateNativeOpenScadGeometry('scale(1/0)cube(1);',{profile:'openscad/stable-2021.01'})
 expect(dropped.ok).toBe(true);if(dropped.ok)expect(dropped.meshes).toHaveLength(0)
})

it('returns stable empty geometry and warnings for singular scale',()=>{
 const result=evaluateNativeOpenScadGeometry('scale([0,1,1])cube(1);',{profile:'openscad/stable-2021.01'})
 expect(result.ok).toBe(true);if(result.ok){expect(result.meshes).toHaveLength(0);expect(result.warnings).toContain('scale() produced empty geometry from a singular transform')}
})

it('executes projective native matrices without discarding their last row',()=>{
 const result=evaluateNativeOpenScadGeometry('multmatrix([[1,0,0,0],[0,1,0,0],[0,0,1,0],[0.1,0,0,1]])cube(1);',{profile:'openscad/stable-2021.01'})
 expect(result.ok).toBe(true)
 if(result.ok)expect(Math.max(...Array.from(result.meshes[0].positions).filter((_,i)=>i%3===0))).toBeCloseTo(1/1.1,6)
})

it('executes stable spheres including the minimum fragment count',async()=>{
 for(const source of ['sphere(r=2,$fn=3);','sphere(r=2,$fn=8);','sphere(r=9,d=4,$fn=9);','sphere(r=0);']){
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Stable sphere failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
 }
})

it('shares projective mesh execution with the stable production evaluator',async()=>{
 const source='multmatrix([[1,0,0,0],[0,1,0,0],[0,0,1,0],[0.1,0,0,1]])cube(1);'
 const result=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
 const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
 expect(native.ok).toBe(true)
 if(native.ok)expect(result.volume).toBeCloseTo(inspectPolygonMesh(native.meshes[0]).signedVolumeMm3,6)
 expect(result.volume).toBeGreaterThan(0)
 const crossing=await parseOpenSCAD('multmatrix([[1,0,0,0],[0,1,0,0],[0,0,1,0],[2,0,0,1]])cube(2,true);',{languageProfile:'openscad/stable-2021.01'})
 expect(crossing.volume).toBe(0)
 expect(crossing.warnings).toContain('multmatrix() transform could not be represented by the geometry kernel')
})

it('executes stable square profiles and extrusion through native programs',async()=>{
 for(const source of ['linear_extrude(3) square(2,true);','linear_extrude(height=3,twist=90,slices=8) square(2,true);','linear_extrude(3) difference(){square(4,true);square(2,true);}','linear_extrude(3) translate([1,2])scale([2,3,0])square(2);','linear_extrude(0)square(2);','linear_extrude(height=3,twist=360,$fn=12)square(2,true);','linear_extrude(height=3,scale=[0.5,2],$fn=8)square(2,true);']){
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native extrusion failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
 }
})

it('preserves executor slice clamp warnings and reduction status',()=>{
 const result=evaluateNativeOpenScadGeometry('linear_extrude(height=3,slices=1000)square(2);',{profile:'openscad/stable-2021.01'})
 expect(result.ok).toBe(true)
 if(result.ok){expect(result.reduced).toBe(true);expect(result.warnings).toContain('linear_extrude slices were clamped to the engine limit 512');expect(result.meshes).toHaveLength(1)}
})

it('executes stable circles through profile extrusion and boolean rings',async()=>{
 for(const source of ['linear_extrude(3)circle(2,$fn=3);','linear_extrude(3)circle(r=9,d=4,$fn=8);','linear_extrude(3)difference(){circle(2,$fn=16);circle(1,$fn=16);}','linear_extrude(height=3,twist=90,$fn=12)translate([2,0])circle(1,$fn=8);','linear_extrude(3)circle(0);']) {
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native circle failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
 }
})

it('executes stable polygon contours, paths and even-odd holes',async()=>{
 for(const source of [
  'linear_extrude(3)polygon([[0,0],[4,0],[0,4]]);',
  'linear_extrude(3)polygon([[0,0],[4,0],[4,4],[0,4],[1,1],[3,1],[3,3],[1,3]],[[0,1,2,3],[4,5,6,7]]);',
  'linear_extrude(3)polygon([[0,0],[4,0],[0,4]],[[0,1,99,2]]);',
  'linear_extrude(3)polygon([[0],[1,0],[0,1]]);',
  'linear_extrude(3)polygon([[0,0],[1,0],[2,0]]);',
  'linear_extrude(3)polygon([[0/0,0],[1,0],[0,1]]);',
 ]) {
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native polygon failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
  for(const warning of reference.warnings)expect(native.warnings,source).toContain(warning)
 }
})

it('executes stable revolutions with signed angles, holes and negative profiles',async()=>{
 for(const source of [
  'rotate_extrude($fn=16)translate([2,0])square([1,3]);',
  'rotate_extrude(angle=90,$fn=16)translate([2,0])square([1,3]);',
  'rotate_extrude(angle=-90,$fn=16)translate([2,0])square([1,3]);',
  'rotate_extrude(angle=90,$fn=16)translate([-3,0])square([1,3]);',
  'rotate_extrude($fn=16)square([2,3]);',
  'rotate_extrude(angle=90,$fn=16)difference(){translate([1,0])square([3,3]);translate([2,1])square(1);}',
  'rotate_extrude(angle=0)square(1);',
  'rotate_extrude($fn=16)circle(0);',
 ]) {
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native revolution failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0),source).toBeCloseTo(reference.volume,5)
  for(const warning of reference.warnings)expect(native.warnings,source).toContain(warning)
 }
})

it('executes stable offsets with shared joins, precedence and nested profiles',async()=>{
 for(const source of [
  'linear_extrude(2)offset(r=1,$fn=16)square(4);',
  'linear_extrude(2)offset(r=1,$fn=1000)square(4);',
  'linear_extrude(2)offset(delta=1,$fn=-2)square(4);',
  'linear_extrude(2)offset(delta=1)square(4);',
  'linear_extrude(2)offset(delta=-1)square(4);',
  'linear_extrude(2)offset(delta=1,chamfer=true)square(4);',
  'linear_extrude(2)offset(r=0,delta=1)square(4);',
  'linear_extrude(2)offset(delta=0.25)difference(){square(4);translate([1,1])square(2);}',
  'linear_extrude(2)offset(r=1,$fn=16)circle(0);',
 ]) {
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native offset failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0),source).toBeCloseTo(reference.volume,5)
  for(const warning of reference.warnings)expect(native.warnings,source).toContain(warning)
 }
})

it('executes native 2D and 3D hull expressions',async()=>{
 for(const source of [
  'hull(){cube(1);translate([3,0,0])cube(1);}',
  'hull(){sphere(1,$fn=8);translate([3,0,0])sphere(1,$fn=8);}',
  'linear_extrude(2)hull(){square(1);translate([3,0])square(1);}',
  'linear_extrude(2)hull(){circle(1,$fn=8);translate([3,0])circle(1,$fn=8);}',
  'linear_extrude(2)hull()square(1);',
  'hull(){}',
 ]) {
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native hull failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0),source).toBeCloseTo(reference.volume,5)
 }
})

it('executes native convex 3D Minkowski expressions',async()=>{
 for(const source of ['minkowski(){cube(1);cube(2);}','minkowski(){translate([2,3,4])cube(1);cube(2);}','minkowski()cube(2);','minkowski(){}']){
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native Minkowski failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0),source).toBeCloseTo(reference.volume,5)
 }
})

it('executes planar Minkowski products entirely through Rust',async()=>{
 for(const source of ['linear_extrude(2)minkowski(){square([2,3]);square([1,2]);}', 'linear_extrude(2)minkowski(){translate([5,-3])square([2,3]);translate([-2,4])square([1,2]);square(1);}', 'linear_extrude(2)minkowski(){circle(1,$fn=8);square(1);}', 'linear_extrude(2)minkowski(){circle(0);square(1);}']){
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native planar Minkowski failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0),source).toBeCloseTo(reference.volume,5)
 }
})

it('executes projection and zero-plane sections through nested Rust programs',async()=>{
 for(const source of [
  'linear_extrude(2)projection()cube([2,3,4]);',
  'linear_extrude(2)projection(cut=true)cube([2,3,4],true);',
  'linear_extrude(2)projection(cut=true)translate([0,0,5])cube(2);',
  'linear_extrude(2)projection(){cube(2);translate([3,0,0])cube(1);}',
  'linear_extrude(2)projection()linear_extrude(3)projection()cube(2);',
  'linear_extrude(2)projection()cube(0);',
 ]) {
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native projection failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
 }
})

it('preserves nested executor warnings across projection',()=>{
 const result=evaluateNativeOpenScadGeometry('linear_extrude(1)projection()rotate_extrude($fn=1000)translate([2,0])square(1);',{profile:'openscad/stable-2021.01',quality:'preview'})
 expect(result.ok).toBe(true)
 if(result.ok){
  expect(result.reduced).toBe(true)
  expect(result.warnings).toContain('Fragment count was clamped to the engine safety limit 48')
 }
})

it('executes deferred planar and solid resize with group extents',async()=>{
 for(const source of [
  'resize([10,0,0],auto=true)cube([5,3,2]);',
  'resize([10,0,0],auto=[false,true,false])translate([2,0,0])cube([5,3,2]);',
  'resize([8,0,0],auto=true){cube(1);translate([3,0,0])cube(1);}',
  'linear_extrude(2)resize([10,0],auto=true)square([5,3]);',
  'linear_extrude(2)resize([8,0],auto=true){square(1);translate([3,0])square(1);}',
  'resize("bad")cube(1);', 'resize("bad")cube(0);',
  'resize([0,-1,"bad"],auto=true)cube([5,3,2]);',
 ]) {
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native resize failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
  expect(native.warnings.includes('resize newsize must be a vector')).toBe(source==='resize("bad")cube(1);')
 }
})

it('executes normalized polyhedra with legacy face warnings and empty failures',async()=>{
 const points='[[0,0,0],[1,0,0],[0,1,0],[0,0,1]]'
 const faces='[[0,1,2],[0,3,1],[0,2,3],[1,3,2]]'
 for(const source of [
  `polyhedron(points=${points},faces=${faces});`,
  `translate([2,3,4])polyhedron(points=${points},triangles=${faces});`,
  `polyhedron(points=${points},faces=[[0,99,1,2],[0,3,1],[0,2,3],[1,3,2]]);`,
  'polyhedron(points=[],faces=[]);',
  'for(i=[0:1])polyhedron(points=[[0,0,0],[1,0,0],[2,0,0]],faces=[[0,1,2]]);',
  'polyhedron(points=[[0,0,0],[1,0,0],[2,0,0]],faces=[[0,1,2]]);',
  'polyhedron(points=[[1e300,0,0],[0,1,0],[0,0,1]],faces=[[0,1,2]]);',
  'polyhedron(points=[[0,0],[1,0],[0,1],["bad",0]],faces=[[0,1,2],[0,3,1]]);',
 ]) {
  const native=evaluateNativeOpenScadGeometry(source,{profile:'openscad/stable-2021.01'})
  const reference=await parseOpenSCAD(source,{languageProfile:'openscad/stable-2021.01'})
  expect(native.ok,source).toBe(true);if(!native.ok)throw Error('Native polyhedron failed')
  expect(native.meshes.reduce((sum,mesh)=>sum+inspectPolygonMesh(mesh).signedVolumeMm3,0)).toBeCloseTo(reference.volume,5)
  expect(native.warnings,source).toEqual(reference.warnings)
 }
 const degenerate=evaluateNativeOpenScadGeometry('polyhedron(points=[[0,0,0],[1,0,0],[2,0,0]],faces=[[0,1,2]]);',{profile:'openscad/stable-2021.01'})
 if(degenerate.ok){expect(degenerate.meshes).toHaveLength(0);expect(degenerate.warnings).toContain('polyhedron() topology did not produce a manifold solid')}
})
