import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { inspectNurbsBrep } from '../src/kernel/geometry/brep'
import { hasSafeSuccessPayload } from '../src/kernel/geometryWorkerProtocol/isGeometryEvaluationResultPayload'
import { readNativeGeometryData } from '../src/kernel/nativeGeometrySnapshot'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

async function openIges(text: string) {
  const source = '// @rush/1\nbody = open_iges_file(iges_text:' + JSON.stringify(text) + ')\nshow body.brep_tessellate(8)\n'
  const scene = await buildTextNurbsScene(compileRushFrontend(source).document)
  return readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
}

it('exports and reopens a rational closed cylinder in IGES', async () => {
  const source = readFileSync('examples/rush/rational-iges-roundtrip.r', 'utf8')
  const scene = await buildTextNurbsScene(compileRushFrontend(source).document)
  const exchange = scene.measurements?.find(report => report.operation === 'export_iges_body')
  expect(exchange?.iges?.split(/\r?\n/).some(line => line[72] === 'D')).toBe(true)
  const payload = { meshes: [], warnings: [], measurements: [exchange] }
  expect(hasSafeSuccessPayload(payload)).toBe(true)
  expect(hasSafeSuccessPayload({ ...payload, measurements: [{ ...exchange, iges: 42 }] })).toBe(false)
  expect(hasSafeSuccessPayload({ ...payload, measurements: [{ ...exchange, iges: '\ud800' }] })).toBe(false)
  expect(JSON.parse(exchange!.report)).toMatchObject({ certificate: { complete: true } })
  expect(scene.meshes).toHaveLength(1)
  const model = readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
  expect(model.bodies).toHaveLength(1)
  expect(inspectNurbsBrep(model)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
  const imported = await openIges(exchange!.iges!)
  expect(imported.faces).toEqual(model.faces)
  expect(model.faces.some(face => face.surface.weights.some(row => row.some(weight => weight !== 1)))).toBe(true)
}, 180000)

it('opens the independently qualified OCCT exchange fixture in the Rush editor', async () => {
  const text = readFileSync('docs/qualification/nurbs-iges-direct-solid-957-2026-11-18/cylinder.igs', 'utf8')
  const model = await openIges(text)
  expect(model.bodies).toHaveLength(1)
  expect(model.faces).toHaveLength(6)
  expect(inspectNurbsBrep(model)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
}, 180000)

it('imports a composite circular rim with rational spline edges', async () => {
  const text = readFileSync('crates/cad-step/tests/fixtures/iges/legacy-spinner-rim.igs', 'utf8')
  const directoryTypes = text.split(/\r?\n/).filter(line => line[72] === 'D')
    .filter((_, index) => index % 2 === 0).map(line => Number(line.slice(0, 8)))
  expect(directoryTypes).toEqual(expect.arrayContaining([100, 102, 126, 128]))
  const model = await openIges(text)
  expect(model.bodies).toHaveLength(1)
  expect(model.faces).toHaveLength(6)
  expect(model.edges.some(edge => edge.curve.weights.some(weight => weight !== 1))).toBe(true)
  expect(inspectNurbsBrep(model)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
}, 180000)

it('reports malformed IGES instead of publishing a scene', async () => {
  await expect(openIges('invalid IGES')).rejects.toThrow(/IGES|record|section/i)
}, 30000)

it.each([
  ['split-rational-edge', [102, 126]],
  ['placed-arc', [100, 102, 124]],
  ['placed-surface', [124, 128]],
] as const)('preserves the rational cylinder from %s', async (fixture, types) => {
  const text = readFileSync('tests/fixtures/rush-iges/' + fixture + '.igs', 'utf8')
  const directoryTypes = text.split(/\r?\n/).filter(line => line[72] === 'D')
    .filter((_, index) => index % 2 === 0).map(line => Number(line.slice(0, 8)))
  expect(directoryTypes).toEqual(expect.arrayContaining(types))
  const model = await openIges(text)
  expect(model.bodies).toHaveLength(1)
  expect(model.faces).toHaveLength(6)
  expect(inspectNurbsBrep(model)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
  const coordinates = model.vertices.map(vertex => vertex.point)
  for (const [axis, low, high] of [[0, -3, 3], [1, -3, 3], [2, 0, 5]] as const) {
    expect(Math.min(...coordinates.map(point => point[axis]!))).toBeCloseTo(low, 9)
    expect(Math.max(...coordinates.map(point => point[axis]!))).toBeCloseTo(high, 9)
  }
}, 180000)

it.each([
  ['nested-arc', 'placed-arc'],
  ['placed-composite', 'placed-arc'],
  ['nested-surface', 'placed-surface'],
] as const)('reconstructs equivalent world geometry for %s', async (fixture, reference) => {
  const fromFixture = (name: string) => openIges(readFileSync('tests/fixtures/rush-iges/' + name + '.igs', 'utf8'))
  const actual = await fromFixture(fixture)
  const expected = await fromFixture(reference)
  expect(inspectNurbsBrep(actual)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
  expect(actual.faces).toEqual(expected.faces)
  expect(actual.edges.map(edge => edge.curve.controlPoints)).toEqual(expected.edges.map(edge => edge.curve.controlPoints))
}, 180000)

it('reconstructs outer edges from the natural boundary of an IGES face', async () => {
  const text = readFileSync('tests/fixtures/rush-iges/natural-boundary.igs', 'utf8')
  const model = await openIges(text)
  expect(model.faces).toHaveLength(6)
  expect(model.edges).toHaveLength(12)
  expect(inspectNurbsBrep(model)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
}, 180000)

it('retains degenerate edges at poles without opening the shell', async () => {
  const text = readFileSync('tests/fixtures/rush-iges/natural-pole.igs', 'utf8')
  const model = await openIges(text)
  expect(model.vertices).toHaveLength(4)
  expect(model.edges.filter(edge => !edge.degenerate)).toHaveLength(6)
  for (const face of model.faces) {
    const outer = model.loops[face.outer]!
    const poles = outer.coedges.filter(coedge => model.edges[coedge.edge]!.degenerate)
    expect(poles).toHaveLength(1)
    const pole = model.edges[poles[0]!.edge]!
    expect(pole.vertices[0]).toBe(pole.vertices[1])
  }
  expect(inspectNurbsBrep(model)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
}, 180000)
