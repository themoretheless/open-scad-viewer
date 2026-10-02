import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {bezierNurbsCurve,guidedLoftNurbsCurves,controlTangentLoftNurbsCurves} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import {compileModelGraphNurbs} from '../src/services/modelGraphNurbsCompiler'
import {importMeshFile} from '../src/services/meshImport'

it('lowers guide references and checks tangent units through Rush',()=>{
 const source=readFileSync('examples/rush/guided-loft-surface.r','utf8')
 const compiled=compileModelGraphText(source)
 expect(compiled.document.nodes.find(n=>n.op==='guided_loft_surface')).toMatchObject({parameters:[0,1],guide_parameters:[.5],start_tangents:[[0,40,40],[0,40,40]]})
 expect(()=>compileModelGraphText(source.replace('guide_parameters: [0.5]','guide_parameters: [0.5mm]'))).toThrow()
 expect(()=>compileModelGraphText(source.replace(',end_tangents: [[0mm,-40mm,40mm],[0mm,-40mm,40mm]]',''))).toThrow()
 expect(()=>compileModelGraphText(source.replace('guides: [g]','guides: [missing]'))).toThrow()
})

it('builds the authored guided loft through the geometry WASM',()=>{
 const source=readFileSync('examples/rush/guided-loft-surface.r','utf8')
 const compiled=compileModelGraphText(source)
 expect(buildOwnNurbs(compiled.document,{action:'build'}).report).toBeDefined()
})

it('matches an independent quadratic with an interior guide through WASM',()=>{
 const sections=[0,2].map(z=>bezierNurbsCurve([[0,0,z],[1,0,z]]))
 const guide=bezierNurbsCurve([[.5,0,0],[.5,1,1],[.5,0,2]])
 const s=guidedLoftNurbsCurves(sections,[2,7],[guide],[.5])
 for(const u of [0,.13,.5,.87,1])for(const v of [0,.17,.5,.83,1]){
  const p=evaluateNurbsSurface(s,u,v).point
  expect(p[0]).toBeCloseTo(u,9);expect(p[1]).toBeCloseTo(2*v*(1-v),9);expect(p[2]).toBeCloseTo(2*v,9)
 }
 expect(()=>guidedLoftNurbsCurves(sections,[2,7],[guide],[.4])).toThrow(/crossings/)
 expect(()=>guidedLoftNurbsCurves(sections,[2,7],[guide],[.5],[[0,0,0],[0,0,0]],[[0,0,0],[0,0,0]])).toThrow(/derivatives/)
})

it('retains a variable rational endpoint tangent field',()=>{
 const w=[1,Math.SQRT1_2,1]
 const sections=[0,7].map(z=>bezierNurbsCurve([[2,0,z],[2,2,z],[0,2,z]],w))
 const start:[number,number,number][]=[[0,0,1],[0,0,3],[0,0,-2]]
 const end:[number,number,number][]=[[0,0,-1],[0,0,2],[0,0,4]]
 const s=controlTangentLoftNurbsCurves(sections,[2,7],start,end)
 for(const u of [0,.17,.5,.83,1])for(const v of [.13,.5,.87]){
  const b=[(1-u)**2,2*u*(1-u),u*u],denom=b.reduce((sum,x,i)=>sum+x*w[i]!,0)
  const field=(t:typeof start)=>b.reduce((sum,x,i)=>sum+x*w[i]!*t[i]![2],0)/denom
  const z=(-2*v**3+3*v*v)*7+(v**3-2*v*v+v)*5*field(start)+(v**3-v*v)*5*field(end)
  expect(evaluateNurbsSurface(s,u,v).point[2]).toBeCloseTo(z,9)
 }
})

it('retains guided-loft authoring through JSON and mesh geometry through OBJ export',async()=>{
 const compiled=compileModelGraphText(readFileSync('examples/rush/guided-loft-surface.r','utf8'))
 const built=buildOwnNurbs(compiled.document,{action:'build'})
 if(!('mesh' in built)||!built.mesh)throw new Error('Missing guided loft mesh')
 const json=buildOwnNurbs(compiled.document,{action:'export',format:'json'})
 if(!('artifact' in json)||!json.artifact)throw new Error('Missing guided loft JSON')
 const restored=compileModelGraphNurbs(JSON.parse(json.artifact.text))
 expect(restored.document_sha256).toBe(compiled.document_sha256)
 const rebuilt=buildOwnNurbs(restored.document,{action:'build'})
 if(!('mesh' in rebuilt)||!rebuilt.mesh)throw new Error('Missing rebuilt mesh')
 expect(Array.from(rebuilt.mesh.positions)).toEqual(Array.from(built.mesh.positions))
 expect(Array.from(rebuilt.mesh.indices)).toEqual(Array.from(built.mesh.indices))
 const obj=buildOwnNurbs(compiled.document,{action:'export',format:'obj'})
 if(!('artifact' in obj)||!obj.artifact||!('base64' in obj.artifact))throw new Error('Missing OBJ')
 const bytes=Uint8Array.from(atob(obj.artifact.base64),c=>c.charCodeAt(0))
 const imported=await importMeshFile('guided-loft.obj',bytes,{weld:false})
 expect(Array.from(imported.indices)).toEqual(Array.from(built.mesh.indices))
 expect(imported.positions.length).toBe(built.mesh.positions.length)
 imported.positions.forEach((x,i)=>expect(Math.abs(x-built.mesh!.positions[i]!)).toBeLessThan(1e-5))
})

it('preserves base boundaries with multiple interior guides through WASM',()=>{
 const sections=[0,2].map(z=>bezierNurbsCurve([[0,0,z],[1,0,z]]))
 const guides=[.25,.75].map(u=>bezierNurbsCurve([[u,0,0],[u,1,1],[u,0,2]]))
 const s=guidedLoftNurbsCurves(sections,[0,1],guides,[.25,.75])
 for(const [u,factor] of [[0,0],[.125,.546875],[.25,1],[.5,1.375],[.75,1],[1,0]])for(const v of [.17,.5,.83]){
  const p=evaluateNurbsSurface(s,u!,v).point
  expect(p[0]).toBeCloseTo(u!,9);expect(p[1]).toBeCloseTo(2*v*(1-v)*factor!,9);expect(p[2]).toBeCloseTo(2*v,9)
 }
})

it('imports a closed loft shared seam through the published WASM STEP route',async()=>{
 const {importDirectStepV9}=await import('../src/services/cadNurbsStep')
 const imported=importDirectStepV9(readFileSync('tests/fixtures/loft/closed-shared-seam.step','utf8'))
 expect(imported.model.edges).toHaveLength(3)
 expect(imported.model.bodies).toHaveLength(0)
 expect(imported.model.edges.every(e=>!e.degenerate)).toBe(true)
 const s=imported.model.faces[0]!.surface
 for(const u of [0,.13,.5,.87,1]){
  const a=evaluateNurbsSurface(s,u,0).point,b=evaluateNurbsSurface(s,u,1).point
  a.forEach((x,i)=>expect(x).toBeCloseTo(b[i]!,9))
 }
})

it('retains authored tangent units through Cartesian Rush and packaged geometry',()=>{
 for(const path of ['examples/rush/cartesian-control-tangent-loft.r','examples/rush/cartesian-auto-control-tangent-loft.r']){
  const source=readFileSync(path,'utf8')
  const compiled=compileModelGraphText(source)
  const loft=compiled.document.nodes.find(n=>n.op==='guided_loft_surface'||n.op==='auto_guided_loft_surface')!
  expect(loft).toMatchObject({construction:'cartesian',parameters:[2,7],max_cells:50000,max_map_evaluations:200000})
  const built=buildOwnNurbs(compiled.document,{action:'build'})
  expect(built.report.construction?.[loft.id]).toMatchObject({exact:false,
   tangents:[{accepted:true,targetUnits:'authored-dP/dt',stationDomain:[2,7]},
             {accepted:true,targetUnits:'authored-dP/dt',stationDomain:[2,7]}]})
  expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('max_map_evaluations: 200000','max_map_evaluations: 1')).document,{action:'build'})).toThrow()
  expect(()=>compileModelGraphText(source.replace('construction: "cartesian"','construction: "homogeneous"'))).toThrow()
  expect(()=>compileModelGraphText(source.replace('parameters: [2,7]','parameters: [2mm,7mm]'))).toThrow()
 }
},60000)

it('retains original section compositions through mapped Rush entrypoints',()=>{
 const source=readFileSync('examples/rush/cartesian-mapped-loft.r','utf8')
 const automatic=source.replace('guided_loft_surface(a,b','auto_guided_loft_surface(a,b').replace('guide_parameters: [0],','').replace('error_budget:','budget:')
 const factor=(u:number)=>{const a=(1-u)**2,b=2*u*(1-u)*.75,c=u*u;return(.2*b+c)/(a+b+c)}
 for(const text of [source,automatic]){
  const compiled=compileModelGraphText(text),loft=compiled.document.nodes.find(n=>n.op==='guided_loft_surface'||n.op==='auto_guided_loft_surface')!
  const built=buildOwnNurbs(compiled.document,{action:'build'})
  const report=built.report.construction?.[loft.id] as any
  expect(report.originalSectionCertificates).toHaveLength(2)
  for(const certificate of report.originalSectionCertificates){
   expect(certificate).toMatchObject({accepted:true,exact:false,targetAuthority:'original-rational-tensor-controls'})
   expect(certificate.errorUpper).toBeLessThanOrEqual(1e-6)
  }
  const surface=built.report.definitions[loft.id] as any
  for(const u of [0,.13,.37,.5,.87,1])for(const [v,weights] of [[0,[1,2]],[1,[3,1]]] as const){
   const t=factor(factor(u)),expected=20*t*weights[1]/((1-t)*weights[0]+t*weights[1])
   expect(evaluateNurbsSurface(surface,u,v).point[0]).toBeCloseTo(expected,9)
  }
  expect(()=>buildOwnNurbs(compileModelGraphText(text.replace('max_map_evaluations: 200000','max_map_evaluations: 1')).document,{action:'build'})).toThrow()
  expect(()=>compileModelGraphText(text.replace('range: [0,1]','range: [0,1mm]'))).toThrow()
  const exported=buildOwnNurbs(compiled.document,{action:'export',format:'json'})
  if(!('artifact' in exported)||!exported.artifact)throw new Error('Missing mapped loft JSON')
  expect(compileModelGraphNurbs(JSON.parse(exported.artifact.text)).document_sha256).toBe(compiled.document_sha256)
 }
 const natural=compileModelGraphText(readFileSync('examples/rush/mapped-natural-loft.r','utf8'))
 const node=natural.document.nodes.find(n=>n.op==='natural_loft_surface')!
 const built=buildOwnNurbs(natural.document,{action:'build'})
 expect(built.report.construction?.[node.id]).toMatchObject({operation:'mapped-natural-loft',exact:false,
  originalSectionCertificates:[{accepted:true},{accepted:true}]})
},60000)

it('builds authored nonplanar caps and refuses incomplete embedding through Rush',()=>{
 const source=readFileSync('examples/rush/authored-nonplanar-cap-loft.r','utf8')
 const compiled=compileModelGraphText(source),built=buildOwnNurbs(compiled.document,{action:'build'})
 expect(built.report.root_kind).toBe('brep')
 const model=built.report.definitions[compiled.document.root] as any
 expect(model.faces).toHaveLength(6)
 expect(model.bodies).toHaveLength(1)
 expect(evaluateNurbsSurface(model.faces[4].surface,.5,.5).point[2]).toBeCloseTo(.125,12)
 expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('loft_embedding_limits()','loft_embedding_limits(facePairs: 1)')).document,{action:'build'})).toThrow(/embedding|pairs|budget/i)
 expect(()=>compileModelGraphText(source.replace('cap_surfaces: [a,b],',''))).toThrow()
 expect(()=>compileModelGraphText(source.replace('tolerance_uv: 0.000000001','tolerance_uv: 0.000000001mm'))).toThrow()
},60000)

it('combines nested section maps, unequal weights, curved guides and authored tangent units',()=>{
 const source=readFileSync('examples/rush/cartesian-mapped-loft.r','utf8')
 // Two quadratic map factors composed with a linear source yield five U controls.
 // The degree-two weighted guide has original dP/dt = (0, +/-6.4, 16)
 // at the ends of the section station interval [2,7].
 const start=JSON.stringify(Array.from({length:5},()=>[0,6.4,16])),end=JSON.stringify(Array.from({length:5},()=>[0,-6.4,16]))
 const guided=source.replace('max_map_evaluations: 200000)',`max_map_evaluations: 200000,start_tangents: ${start},end_tangents: ${end})`)
 const automatic=guided.replace('guided_loft_surface(a,b','auto_guided_loft_surface(a,b').replace('guide_parameters: [0],','').replace('error_budget:','budget:')
 for(const text of [guided,automatic]){
  const compiled=compileModelGraphText(text),node=compiled.document.nodes.find(n=>n.op==='guided_loft_surface'||n.op==='auto_guided_loft_surface')!
  const built=buildOwnNurbs(compiled.document,{action:'build'}),report=built.report.construction?.[node.id] as any
  expect(report.originalSectionCertificates.every((c:any)=>c.accepted&&c.errorUpper<=1e-6)).toBe(true)
  expect(report.tangents).toMatchObject([{accepted:true,targetUnits:'authored-dP/dt'},{accepted:true,targetUnits:'authored-dP/dt'}])
  for(const u of [0,.13,.37,.83,1])for(const [v,y] of [[0,32],[1,-32]]){
   const dv=evaluateNurbsSurface(built.report.definitions[node.id] as any,u,v).dv!
   expect(dv[0]).toBeCloseTo(0,8);expect(dv[1]).toBeCloseTo(y,8);expect(dv[2]).toBeCloseTo(80,8)
  }
 }
},60000)
