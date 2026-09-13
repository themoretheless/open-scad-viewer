import {expect, it} from 'vitest'
import {isNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {assertGeometryScene, geometrySceneFromMeshes, meshesFromGeometryScene} from '../src/core/scene'
import {DirectHistory, directBodiesScad, emptyDirectDocument, parseDirectDocument} from '../src/services/directModeling'
import {booleanNurbsBrep, createBrepBox, tessellateNurbsBrep} from '../src/services/geometry/brep'
import {meshToNurbsBrep} from '../src/services/geometry/reconstruction'
import {isGeometryEvaluationResultPayload} from '../src/services/geometryWorkerProtocol'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {sceneMeshesToSolidDocument} from '../src/services/solidBridge'

const emptyGraph = (display: boolean) => ({
  language: 'modelgraph/nurbs-1', units: 'mm',
  nodes: [
    {id: 'box', op: 'brep_box', min: [0, 0, 0], max: [2, 3, 4]},
    {id: 'zero', op: 'brep_boolean', inputs: ['box', 'box'], operation: 'difference'},
    ...(display ? [{id: 'display', op: 'brep_tessellate', input: 'zero', segments: 4}] : []),
  ],
  root: display ? 'display' : 'zero',
})

it('publishes empty B-rep text as zero scene entities and zero Solid bodies', async () => {
  for (const suffix of ['', '.brep_tessellate(4)']) {
    for (const quality of ['preview', 'full'] as const) {
      const result = await parseOpenSCAD('// @modelgraph-text/1\na=brep_box([0,0,0],[2mm,3mm,4mm])\nshow a.brep_subtract(a)' + suffix, {quality})
      expect(result.meshes).toEqual([])
      expect(result.warnings).toEqual([])
      expect(result.volume).toBe(0)
      expect(result.surfaceArea).toBe(0)
      expect(isGeometryEvaluationResultPayload(structuredClone(result))).toBe(true)
      const scene = geometrySceneFromMeshes(result.meshes)
      expect(() => assertGeometryScene(scene)).not.toThrow()
      expect(scene.assets).toEqual([])
      expect(scene.entities).toEqual([])
      expect(meshesFromGeometryScene(scene)).toEqual([])
      const solid = sceneMeshesToSolidDocument(result.meshes)
      expect(parseDirectDocument(JSON.stringify(solid)).bodies).toEqual([])
      expect(directBodiesScad(solid)).toBe('')
    }
  }
})

it('retains a valid empty native snapshot independently of display resolution', () => {
  for (const explicitDisplay of [false, true]) {
    const document = emptyGraph(explicitDisplay)
    let revision: string | undefined
    for (const segments of [1, 8, 32]) {
      const result = buildOwnNurbs(document, {action: 'build', display: {segments, subdivisionLevels: 1}})
      if (!('nativeGeometry' in result) || !('mesh' in result) || !result.mesh) throw Error('Missing native display result')
      expect(result.report.bounds).toBeNull()
      expect(result.mesh.positions).toEqual([])
      expect(result.mesh.indices).toEqual([])
      expect(result.mesh.report.closed).toBe(false)
      expect(isNativeGeometryArtifact(result.nativeGeometry)).toBe(true)
      expect(result.nativeGeometry.kind).toBe('brep')
      expect(result.nativeGeometry.nodeId).toBe('zero')
      const retained = JSON.parse(result.nativeGeometry.geometryJson).geometry
      for (const collection of ['vertices', 'edges', 'loops', 'faces', 'shells', 'bodies']) expect(retained[collection]).toEqual([])
      if (revision) expect(result.nativeGeometry.revision).toBe(revision)
      revision = result.nativeGeometry.revision
    }
  }
})

it('round-trips an empty Boolean program through native JSON export', () => {
  const result = buildOwnNurbs(emptyGraph(true), {action: 'export', format: 'json'})
  if (!('artifact' in result) || !result.artifact || !('text' in result.artifact)) throw Error('Missing JSON artifact')
  const restored = buildOwnNurbs(JSON.parse(result.artifact.text), {action: 'build'})
  expect(restored.document_sha256).toBe(result.document_sha256)
  expect(restored.report.bounds).toBeNull()
  expect('mesh' in restored && restored.mesh?.indices).toEqual([])
})

it('exports zero-element open mesh formats and refuses printing formats for empty B-reps', () => {
  for (const format of ['obj', 'ply', 'off'] as const) {
    const result = buildOwnNurbs(emptyGraph(true), {action: 'export', format})
    if (!('artifact' in result) || !result.artifact || !('base64' in result.artifact)) throw Error('Missing mesh artifact')
    const text = atob(result.artifact.base64)
    expect(result.report.bounds).toBeNull()
    expect(text).not.toMatch(/Infinity|NaN/)
    if (format === 'obj') expect(text).not.toMatch(/^[vf] /m)
    if (format === 'ply') expect(text).toContain('element vertex 0\n')
    if (format === 'ply') expect(text).toContain('element face 0\n')
    if (format === 'off') expect(text).toContain('OFF\n0 0 0\n')
  }
  for (const format of ['stl', 'stl_binary', '3mf', 'amf'] as const) {
    expect(() => buildOwnNurbs(emptyGraph(true), {action: 'export', format})).toThrow(/closed.*positive volume/i)
  }
})

it('rejects a saved phantom body with an empty authoritative B-rep and stale display triangles', () => {
  const box = createBrepBox([0, 0, 0], [2, 3, 4])
  const empty = booleanNurbsBrep(box, box, 'difference')
  const display = tessellateNurbsBrep(box, 2)
  const stale = {version: 1 as const, sketches: [], bodies: [{id: 'stale', name: 'Stale body', brep: empty, mesh: {positions: display.positions, indices: display.indices}}]}
  expect(() => parseDirectDocument(JSON.stringify(stale))).toThrow(/empty.*B-rep|B-rep.*empty/i)
  expect(() => new DirectHistory(stale)).toThrow(/empty.*B-rep|B-rep.*empty/i)
  const history = new DirectHistory(emptyDirectDocument())
  expect(history.document.bodies).toEqual([])
})

it('preserves a nonempty open B-rep sheet when it has no volume body', () => {
  const mesh = {positions: [0, 0, 0, 1, 0, 0, 0, 1, 0], indices: [0, 1, 2]}
  const brep = meshToNurbsBrep(mesh)
  expect(brep.bodies).toEqual([])
  expect(brep.faces).toHaveLength(1)
  expect(tessellateNurbsBrep(brep).report.closed).toBe(false)
  const document = parseDirectDocument(JSON.stringify({version: 1, sketches: [], bodies: [{id: 'sheet', name: 'Sheet', brep, mesh}]}))
  expect(document.bodies).toHaveLength(1)
  expect(new DirectHistory(document).document.bodies[0].brep).toEqual(brep)
})
