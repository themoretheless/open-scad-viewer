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
