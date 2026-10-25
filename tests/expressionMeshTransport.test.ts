import {readFileSync} from 'node:fs'
import {describe,it,expect} from 'vitest'
import {validExpressionMesh,type ExpressionMeshReport} from '../src/services/expressionMesh'
import {expressionShellExpectation,validExpressionShell,type ExpressionShellOptions} from '../src/services/expressionShell'
import {expressionInspectionExpectation,validExpressionInspection} from '../src/services/expressionShellInspection'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-mesh-native-2026-10-09.json',import.meta.url),'utf8'))
const {op:_,...options}=fixture.request as ExpressionShellOptions&{op:string}
const limits=options.meshLimits!,faces=fixture.result.shell.faceCount
const fresh=()=>structuredClone(fixture.result.shell.displayMesh) as ExpressionMeshReport
describe('native adaptive expression mesh transport',()=>{
 it('accepts the extended-budget production worker result and binds its limits',()=>{
  const report=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-mesh-extended-wasm-2026-10-09/expression-inspection-worker-report.json',import.meta.url),'utf8'))
  const response=report.retry,request=response.request
  expect(request.meshLimits.cells).toBe(32768)
  expect(validExpressionInspection(expressionInspectionExpectation(request),response)).toBe(true)
  expect(response.shell.displayMesh.spans).toBe(request.meshLimits.spans)
  expect(response.shell.displayMesh.domainCells).toBe(request.meshLimits.domainCells)
  const changed=structuredClone(request);changed.meshLimits.domainCells=30000
  expect(validExpressionInspection(expressionInspectionExpectation(changed),response)).toBe(false)
 })
 it('preserves the native 0.05 mm bound and refuses a stale coarse response',()=>{
  const precise=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-mesh-precision-native-2026-10-09.json',import.meta.url),'utf8'))
  const {op:_,...request}=precise.request as ExpressionShellOptions&{op:string}
  expect(request.meshLimits!.maxDeviationMm).toBe(0.05)
  expect(validExpressionShell(expressionShellExpectation(request),precise.result)).toBe(true)
  expect(validExpressionShell(expressionShellExpectation(request),fixture.result)).toBe(false)
  const mesh=structuredClone(precise.result.shell.displayMesh) as ExpressionMeshReport
  expect(mesh.tiles.some(tile=>precise.result.shell.faces[tile.face].kind==='transition')).toBe(true)
  expect(mesh.pending.length).toBeGreaterThan(0)
  mesh.tiles[0].deviationUpperMm=0.050001
  expect(validExpressionMesh(request.meshLimits!,precise.result.shell.faceCount,mesh)).toBe(false)
 })
 it('accepts actual native partial mesh without admitting a watertight solid',()=>{
  expect(validExpressionMesh(limits,faces,fresh())).toBe(true)
  expect(validExpressionShell(expressionShellExpectation(options),fixture.result)).toBe(true)
  expect(fixture.result.materialQualified).toBe(false)
  expect(fixture.result.shell.displayMesh.complete).toBe(false)
  expect(validExpressionMesh(limits,faces,{...fresh(),watertightQualified:true})).toBe(false)
  expect(validExpressionMesh(limits,faces,{...fresh(),complete:true})).toBe(false)
 })
 it('rejects lost leaves, duplicate domains and work counter corruption',()=>{
  const missing=fresh();missing.pending.pop()
  expect(validExpressionMesh(limits,faces,missing)).toBe(false)
  const duplicate=fresh();duplicate.tiles[1]=structuredClone(duplicate.tiles[0])
  expect(validExpressionMesh(limits,faces,duplicate)).toBe(false)
  expect(validExpressionMesh(limits,faces,{...fresh(),splitCells:limits.cells+1})).toBe(false)
  expect(validExpressionMesh(limits,faces,{...fresh(),spans:limits.spans+1})).toBe(false)
 })
 it('rejects deviation and coordinate corruption',()=>{
  const inaccurate=fresh();inaccurate.tiles[0].deviationUpperMm=limits.maxDeviationMm+1
  expect(validExpressionMesh(limits,faces,inaccurate)).toBe(false)
  const nonfinite=fresh();nonfinite.tiles[0].corners[0][0]=Infinity
  expect(validExpressionMesh(limits,faces,nonfinite)).toBe(false)
  expect(validExpressionMesh(limits,faces,{...fresh(),maxDeviationUpperMm:0})).toBe(false)
 })
 it('binds the full mesh request and rejects unsolicited triangles',()=>{
  const expected=expressionShellExpectation(options)
  const changed=structuredClone(options);changed.meshLimits!.maxDepth++
  expect(validExpressionShell(expected,{...fixture.result,request:changed})).toBe(false)
  const {meshLimits:_,...withoutMesh}=options
  expect(validExpressionShell(expressionShellExpectation(withoutMesh),{...fixture.result,request:withoutMesh})).toBe(false)
 })
})
