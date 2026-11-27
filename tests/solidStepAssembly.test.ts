import { afterEach, expect, it, vi } from 'vitest'
import { exportSolidStepAssembly } from '../src/kernel/solidStepAssembly'
import { cadRoadmapParts } from '../benchmarks/cad-roadmap-fixtures'
import * as kernel from '../src/kernel/geometry/kernel'
import type { DirectDocument } from '../src/app/directModeling'
afterEach(() => vi.restoreAllMocks())
const scene = (): DirectDocument => ({
  version: 1,
  sketches: [],
  bodies: cadRoadmapParts().map(f => ({ ...f.body, group: 'Parts' })),
})
it('exports every current body under one root and group without changing the document', async () => {
  const doc = scene(),
    before = structuredClone(doc)
  doc.bodies[0].name = "Кронштейн 'A' #999 $&"
  before.bodies[0].name = doc.bodies[0].name
  const text = await exportSolidStepAssembly(doc)
  expect(text.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(4)
  expect(text.match(/=NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)).toHaveLength(5)
  expect(text).toContain("''A'' #999 $&")
  expect(text).toContain('\\X2\\041A\\X0\\')
  const ids = Array.from(text.matchAll(/^#(\d+)=/gm), m => m[1])
  expect(new Set(ids).size).toBe(ids.length)
  expect(doc).toEqual(before)
})
it('snapshots the scene before asynchronous kernel readiness', async () => {
  const doc = scene()
  let release!: () => void
  vi.spyOn(kernel, 'warmGeometryKernel').mockReturnValue(
    new Promise<void>(resolve => {
      release = resolve
    }),
  )
  const exporting = exportSolidStepAssembly(doc)
  doc.bodies.length = 0
  release()
  expect((await exporting).match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(4)
})
it('refuses partial exports and duplicate identities', async () => {
  const doc = scene()
  delete doc.bodies[1].brep
  await expect(exportSolidStepAssembly(doc)).rejects.toThrow('flange')
  const duplicate = scene()
  duplicate.bodies[1].id = duplicate.bodies[0].id
  await expect(exportSolidStepAssembly(duplicate)).rejects.toThrow('unique body IDs')
  await expect(exportSolidStepAssembly({ version: 1, sketches: [], bodies: [] })).rejects.toThrow('at least one')
  const sketches = scene()
  sketches.sketches.push({
    id: 's',
    name: 's',
    points: [
      [0, 0],
      [1, 0],
    ],
    closed: false,
  })
  await expect(exportSolidStepAssembly(sketches)).rejects.toThrow('body-only')
})

it('shares a linked Solid body across named placements and separates a detached copy', async () => {
  const { createSolidInstance, detachSolidInstances } = await import('../src/kernel/solidInstances')
  const source = scene().bodies[0]
  delete source.group
  delete source.material
  const matrix = [[1,0,0,17],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
  const doc = createSolidInstance({ version: 1, sketches: [], bodies: [source] }, source.id, 'copy', matrix)
  doc.bodies[1].name = "Copy 'A'"
  const shared = await exportSolidStepAssembly(doc)
  expect(shared.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(1)
  expect(shared).toContain("NEXT_ASSEMBLY_USAGE_OCCURRENCE('copy','Copy ''A'''")
  const detached = await exportSolidStepAssembly(detachSolidInstances(doc, ['copy']))
  expect(detached.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(2)
})

it('retains grouped shared placements, empty groups and group identities in STEP', async () => {
  const { createSolidInstance } = await import('../src/kernel/solidInstances')
  const { importDirectStepV9 } = await import('../src/kernel/cadNurbsStep')
  const source = scene().bodies[0]
  delete source.material
  const matrix = [[1,0,0,17],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
  const doc = createSolidInstance({ version: 1, sketches: [], bodies: [source] }, source.id, 'copy', matrix)
  doc.bodies[1].group = "Rotor's"
  doc.groups = [{name: 'Spare', source: 'test'}]
  const text = await exportSolidStepAssembly(doc)
  expect(text.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(1)
  expect(text.match(/=NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)).toHaveLength(5)
  for (const group of ['Spare', 'Parts', "Rotor''s"]) {
    expect(text).toContain(`PRODUCT('group:${group}','${group}'`)
    expect(text).toContain(`NEXT_ASSEMBLY_USAGE_OCCURRENCE('group:${group}','${group}'`)
  }
  expect(importDirectStepV9(text).model.bodies).toHaveLength(2)
})

it('preserves groups when an older shared writer ignores hierarchy fields', async () => {
  const { createSolidInstance } = await import('../src/kernel/solidInstances')
  const step = await import('../src/kernel/cadNurbsStep')
  const writeFlat = step.exportSharedStepAssembly
  vi.spyOn(step, 'exportSharedStepAssembly').mockImplementation((defs, placements) => writeFlat(defs, placements))
  const source = scene().bodies[0]
  delete source.material
  const matrix = [[1,0,0,17],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
  const doc = createSolidInstance({ version: 1, sketches: [], bodies: [source] }, source.id, 'copy', matrix)
  const text = await exportSolidStepAssembly(doc)
  expect(step.exportSharedStepAssembly).toHaveBeenCalledOnce()
  expect(text).toContain("PRODUCT('group:Parts','Parts'")
  expect(text.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(2)
})

it('retains a shared definition and occurrence colours through two mounted groups', async () => {
  const { createSolidInstance } = await import('../src/kernel/solidInstances')
  const { importDirectStepV9 } = await import('../src/kernel/cadNurbsStep')
  const source = scene().bodies[0]
  delete source.group
  source.material = { name: 'Source red', color: '#ff0000' }
  const matrix = [[1,0,0,7],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
  const doc = createSolidInstance({ version: 1, sketches: [], bodies: [source] }, source.id, 'copy', matrix)
  doc.groups = [{name: 'Inner', source: '', parent:'Outer'}, {name:'Outer', source:''}]
  doc.bodies[1].group = 'Inner'
  doc.bodies[1].material = { name: 'Instance blue', color: '#0000ff' }
  const text = await exportSolidStepAssembly(doc)
  expect(text.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(1)
  expect(text.match(/=PRESENTATION_STYLE_BY_CONTEXT\(/g)).toHaveLength(2)
  expect(importDirectStepV9(text).model.bodies).toHaveLength(2)
  const inner = /^#\d+=NEXT_ASSEMBLY_USAGE_OCCURRENCE\('group:Inner','Inner','',#(\d+),#(\d+),\$\)/m.exec(text)!
  const outer = /^#\d+=NEXT_ASSEMBLY_USAGE_OCCURRENCE\('group:Outer','Outer','',#(\d+),#(\d+),\$\)/m.exec(text)!
  expect(inner[1]).toBe(outer[2])
  expect(inner[1]).not.toBe(outer[1])
  const { detachSolidInstances } = await import('../src/kernel/solidInstances')
  const materialized = await exportSolidStepAssembly(detachSolidInstances(doc, ['copy']))
  expect(materialized.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(2)
  expect(materialized.match(/=COLOUR_RGB\(/g)).toHaveLength(2)
  expect(importDirectStepV9(materialized).model.bodies).toHaveLength(2)
  const materializedInner = /^#\d+=NEXT_ASSEMBLY_USAGE_OCCURRENCE\('group:Inner','Inner','',#(\d+),#(\d+),\$\)/m.exec(materialized)!
  const materializedOuter = /^#\d+=NEXT_ASSEMBLY_USAGE_OCCURRENCE\('group:Outer','Outer','',#(\d+),#(\d+),\$\)/m.exec(materialized)!
  expect(materializedInner[1]).toBe(materializedOuter[2])
  doc.groups[1].parent = 'Inner'
  await expect(exportSolidStepAssembly(doc)).rejects.toThrow(/hierarchy/)
})

it('round-trips parent links in the editor document and rejects missing parents or cycles', async () => {
  const { parseDirectDocument, serializeDirectDocument } = await import('../src/app/directModeling')
  const doc: DirectDocument = {
    version: 1, sketches: [], bodies: [],
    groups: [{name: 'Outer', source: ''}, {name:'Inner', source:'', parent:'Outer'}],
  }
  const restore = () => parseDirectDocument(serializeDirectDocument(doc))
  expect(restore().groups).toEqual(doc.groups)
  doc.groups![1]!.parent = 'Missing'
  expect(restore).toThrow(/missing parent/)
  doc.groups![1]!.parent = 'Outer'
  doc.groups![0]!.parent = 'Inner'
  expect(restore).toThrow(/cycle/)
})
