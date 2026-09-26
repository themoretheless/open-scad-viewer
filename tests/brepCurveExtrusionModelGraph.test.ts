import {readFileSync} from 'node:fs'
import {expect, it} from 'vitest'
import {z} from 'zod/v4'
import {compileModelGraphNurbs, modelGraphNurbsSchema} from '../src/services/modelGraphNurbs'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import {analyzeNurbsBrep, inspectNurbsBrep, type NurbsBrep} from '../src/services/geometry/brep'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {ownNurbsLanguage} from '../src/mcp/modelGraphNurbsTools'

const circle = (id: string, radius: number) => ({
  id, op: 'curve', degree: 2,
  knots: [0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],
  control_points: [[radius,0],[radius,radius],[0,radius],[-radius,radius],[-radius,0],[-radius,-radius],[0,-radius],[radius,-radius],[radius,0]],
  weights: [1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1],
})
const document = (nodes: unknown[], root = 'body') => ({language:'modelgraph/nurbs-1',units:'mm',nodes,root})
const body = (loops: string[][]) => ({id:'body',op:'brep_extrude_curves',loops,z_min:-2,z_max:3})
function definition(result: ReturnType<typeof buildOwnNurbs>, name: string): NurbsBrep {
  const {kind, ...model} = result.report.definitions[name]
  expect(kind).toBe('brep')
  return model as NurbsBrep
}
function checkRing(model: NurbsBrep, expected = 40 * Math.PI) {
  expect(inspectNurbsBrep(model).topologyValid).toBe(true)
  expect(model.bodies).toHaveLength(1)
  expect(model.faces.filter(face => face.holes.length)).toHaveLength(2)
  expect(model.edges.some(edge => edge.curve.degree === 2 && edge.curve.weights.some(weight => weight !== 1))).toBe(true)
  expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo(expected, 6)
}

it('exports the nested curve-reference schema and retains full-circle and edited-hole definitions', () => {
  const schema = JSON.parse(readFileSync('docs/languages/modelgraph-nurbs-1.schema.json','utf8'))
  expect(schema).toEqual(z.toJSONSchema(modelGraphNurbsSchema))
  expect(ownNurbsLanguage().schema).toEqual(schema)
  expect(ownNurbsLanguage().guide).toContain('brep_extrude_curves')
  const input = {
    ...document([
      circle('outer',3), circle('hole_source',1),
      {id:'hole',op:'curve_edit',input:'hole_source',reverse:true},
      {...body([['outer'],['hole']]),z_max:{param:'top'}},
      {id:'placed',op:'transform',input:'body',matrix:[[0,-1,0,7],[1,0,0,-4],[0,0,1,2],[0,0,0,1]]},
      {id:'display',op:'brep_tessellate',input:'placed',segments:8},
    ],'display'), parameters:[{id:'top',value:3}],
  }
  const original = JSON.stringify(input)
  const result = buildOwnNurbs(input,{action:'build'})
  checkRing(definition(result,'body'))
  checkRing(definition(result,'placed'))
  expect(result.report.definitions.outer).toMatchObject({controlPoints:circle('outer',3).control_points,weights:circle('outer',3).weights})
  expect(result.mesh?.report.closed).toBe(true)
  expect(JSON.stringify(input)).toBe(original)
  const exported = buildOwnNurbs(input,{action:'export',format:'json'})
  expect('artifact' in exported).toBe(true)
  if (!('artifact' in exported) || !exported.artifact || !('text' in exported.artifact)) throw new Error('Missing native document export')
  const saved = JSON.parse(exported.artifact.text)
  expect(saved.nodes.find((node: {id:string}) => node.id === 'body').loops).toEqual([['outer'],['hole']])
  expect(saved.nodes.find((node: {id:string}) => node.id === 'body').z_max).toEqual({param:'top'})
})

const arc = (name: string, points: number[][]) => `${name}=nurbs_curve(2,[0,0,0,1,1,1],${JSON.stringify(points)},[1,${Math.SQRT1_2},1])`
const inner = circle('hole',1)
const curveText = [
  '// @modelgraph-text/1',
  arc('a',[[3,0],[3,3],[0,3]]),arc('b',[[0,3],[-3,3],[-3,0]]),
  arc('c',[[-3,0],[-3,-3],[0,-3]]),arc('d',[[0,-3],[3,-3],[3,0]]),
  `hole=nurbs_curve(2,${JSON.stringify(inner.knots)},${JSON.stringify([...inner.control_points].reverse())},${JSON.stringify(inner.weights)})`,
  'body=brep_extrude_curves([[a,b,c,d],[hole]],z_min:-2mm,z_max:3mm)',
  'show body.transform([[1,0,0,7mm],[0,1,0,-4mm],[0,0,1,2mm],[0,0,0,1]]).brep_tessellate(8)',
].join('\n')

it('builds text quarter-arc loops and a full-circle hole through the actual viewer path', async () => {
  const compiled = compileModelGraphText(curveText)
  expect(compiled.execution_target).toBe('own-nurbs')
  const result = buildOwnNurbs(compiled.document,{action:'build'})
  const node = compiled.document.nodes.find(node => node.op === 'brep_extrude_curves')!
  expect(node).toMatchObject({loops:[['n1','n2','n3','n4'],['n5']],z_min:-2,z_max:3})
  checkRing(definition(result,node.id))
  const scene = await parseOpenSCAD(curveText)
  expect(scene.meshes).toHaveLength(1)
  expect(scene.meshes[0].faceIdsAuthoritative).toBe(true)
  expect(scene.meshes[0].nativeGeometry?.kind).toBe('brep')
  const native = JSON.parse(scene.meshes[0].nativeGeometry!.geometryJson).geometry
  checkRing(native)
})

it('supports native empty loops in both graph and text without phantom display geometry', async () => {
  const result = buildOwnNurbs(document([body([]),{id:'display',op:'brep_tessellate',input:'body',segments:1}],'display'),{action:'build'})
  expect(definition(result,'body').bodies).toEqual([])
  expect(result.mesh?.indices.length).toBe(0)
  expect(result.report.bounds).toBeNull()
  const scene = await parseOpenSCAD('// @modelgraph-text/1\nshow brep_extrude_curves([],0mm,2mm).brep_tessellate(1)')
  expect(scene.meshes).toEqual([])
  expect(scene.volume).toBe(0)
})

it('preserves mixed circular arcs and nonuniform rational line parameterizations', () => {
  const result = buildOwnNurbs(document([
    {id:'arc',op:'curve',degree:2,knots:[0,0,0,1,1,1],control_points:[[3,0],[3,3],[0,3]],weights:[1,Math.SQRT1_2,1]},
    {id:'left',op:'curve',degree:1,knots:[0,0,1,1],control_points:[[0,3],[0,0]],weights:[1,2]},
    {id:'bottom',op:'curve',degree:1,knots:[0,0,1,1],control_points:[[0,0],[3,0]],weights:[3,1]},
    body([['arc','left','bottom']]),
  ]),{action:'build'})
  const model = definition(result,'body')
  expect(model.faces).toHaveLength(5)
  expect(inspectNurbsBrep(model).topologyValid).toBe(true)
  expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo(45*Math.PI/4,6)
  expect(model.edges.some(edge=>edge.curve.degree===1 && edge.curve.weights[0]!==edge.curve.weights[1])).toBe(true)
})

it('checks nested references, scalar dimensions, runtime curve kinds and active-span budgets', () => {
  const outer = circle('outer',3)
  expect(() => compileModelGraphNurbs(document([outer,body([['missing']])]))).toThrow(/Unknown node/)
  expect(() => compileModelGraphNurbs(document([body([['body']])]))).toThrow(/Cyclic/)
  expect(() => compileModelGraphNurbs(document([outer,body([Array(128).fill('outer'),Array(127).fill('outer')])]))).toThrow(/254/)
  expect(() => buildOwnNurbs(document([outer,body([Array(64).fill('outer')])]),{action:'build'})).toThrow(/254.*active|active.*254/)
  expect(() => buildOwnNurbs(document([{id:'wrong',op:'brep_box',min:[0,0,0],max:[1,1,1]},body([['wrong']])]),{action:'build'})).toThrow(/Expected curve/)
  expect(() => buildOwnNurbs(document([{...outer,control_points:outer.control_points.map(point=>[...point,0])},body([['outer']])]),{action:'build'})).toThrow(/2D/)
  expect(() => buildOwnNurbs(document([{...outer,weights:Array(9).fill(1)},body([['outer']])]),{action:'build'})).toThrow(/circular|circle|quadratic/i)
  expect(() => compileModelGraphText(curveText.replace('z_max:3mm','z_max:3deg'))).toThrow(/Expected/)
  expect(() => compileModelGraphText('// @modelgraph-text/1\nshow brep_extrude_curves([[]],0mm,2mm)')).toThrow(/nonempty/)
})
