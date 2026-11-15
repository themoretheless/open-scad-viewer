import { expect, it } from 'vitest'
import { cloneDocumentTree } from '../src/app/documentClone'
import { sameMeshJson } from '../src/exchange/sameMeshJson'
import { stringifyMeshJson } from '../src/exchange/meshJson'
import {
  DirectHistory,
  emptyDirectDocument,
  parseDirectDocument,
  serializeDirectDocument,
  serializeDirectDraft,
  type DirectBody,
  type DirectDocument,
} from '../src/app/directModeling'
import { createBrepBox, tessellateNurbsBrep } from '../src/kernel/geometry/brep'
import { warmGeometryKernel } from '../src/kernel/geometry/kernel'
import { resolveSolidInstances } from '../src/kernel/solidInstances'
import { SolidInstanceBatchCache } from '../src/kernel/solidInstanceBatchCache'
import { prepareDocumentCommit } from '../src/features/directModeler/prepareDocumentCommit'

const label = (ru: string) => ru
const placement = (x: number) => [
  [1, 0, 0, x],
  [0, 1, 0, 0],
  [0, 0, 1, 0],
  [0, 0, 0, 1],
]
async function linkedScene(instances: number): Promise<DirectDocument> {
  await warmGeometryKernel()
  const brep = createBrepBox([0, 0, 0], [2, 3, 4]),
    mesh = tessellateNurbsBrep(brep, 1)
  const document = emptyDirectDocument()
  document.bodies.push({ id: 'source', name: 'Source', brep, mesh })
  for (let i = 0; i < instances; i++)
    document.bodies.push({
      id: `linked-${i}`,
      name: `Linked ${i}`,
      mesh: { positions: [], indices: [] },
      instance: { sourceId: 'source', matrix: placement(10 + 5 * i) },
    } as DirectBody)
  return parseDirectDocument(serializeDirectDocument(document))
}

it('copies document trees like structuredClone without sharing any node', () => {
  const tree = {
    version: 1,
    numbers: [1, -0, 2.5],
    mixed: [1, 'a', null, { k: [3] }],
    typed: new Float64Array([1, 2, 3]),
    indices: new Uint32Array([0, 1, 2]),
    nested: { a: { b: [[1, 2], [3]] }, missing: undefined },
    date: new Date(5),
  }
  const copy = cloneDocumentTree(tree)
  expect(copy).toStrictEqual(structuredClone(tree))
  expect(Object.is(copy.numbers[1], -0)).toBe(true)
  expect(copy.typed).toBeInstanceOf(Float64Array)
  expect(copy.typed).not.toBe(tree.typed)
  expect(copy.nested.a.b[0]).not.toBe(tree.nested.a.b[0])
  expect(copy.date).not.toBe(tree.date)
  expect('missing' in copy.nested).toBe(true)
  copy.typed[0] = 9
  copy.nested.a.b[0]![0] = 9
  expect(tree.typed[0]).toBe(1)
  expect(tree.nested.a.b[0]![0]).toBe(1)
})

it('compares mesh JSON exactly like the serialized strings', () => {
  const cases: Array<[unknown, unknown]> = [
    [
      { a: 1, b: [1, 2] },
      { a: 1, b: [1, 2] },
    ],
    [
      { a: 1, b: 2 },
      { b: 2, a: 1 },
    ],
    [{ a: 1, b: undefined }, { a: 1 }],
    [[undefined], [null]],
    [[NaN], [null]],
    [[Infinity], [-Infinity]],
    [[0], [-0]],
    [new Float64Array([1, 2]), [1, 2]],
    [new Float64Array([1, 2]), [1, 3]],
    [{ m: { positions: new Float32Array([0.5]) } }, { m: { positions: [0.5] } }],
    [{ a: [1, [2, { c: 'x' }]] }, { a: [1, [2, { c: 'y' }]] }],
    [{ a: null }, { a: NaN }],
    [{ a: 'x' }, { a: 'x', b: () => 1 }],
    [{ a: new Date(1) }, { a: '1970-01-01T00:00:00.001Z' }],
    [
      [1, 2],
      [1, 2, 3],
    ],
    [{}, []],
  ]
  for (const [a, b] of cases)
    expect(sameMeshJson(a, b), stringifyMeshJson(a) + ' vs ' + stringifyMeshJson(b)).toBe(
      stringifyMeshJson(a) === stringifyMeshJson(b),
    )
})

it('resolves instances per placement: moving one instance reuses every other cached result', async () => {
  const document = await linkedScene(40)
  const cache = new SolidInstanceBatchCache()
  const first = resolveSolidInstances(structuredClone(document), cache)
  expect(cache.size).toBe(40)
  const moved = structuredClone(document)
  moved.bodies[7]!.instance!.matrix = placement(500)
  const cached = resolveSolidInstances(structuredClone(moved), cache)
  expect(cache.size).toBe(41)
  // Identical to an uncached resolution, and the untouched instances are unchanged.
  expect(stringifyMeshJson(cached)).toBe(stringifyMeshJson(resolveSolidInstances(structuredClone(moved))))
  expect(stringifyMeshJson(cached.bodies[8])).toBe(stringifyMeshJson(first.bodies[8]))
  // Hits are private copies.
  cached.bodies[8]!.mesh.positions[0] = 999
  expect(resolveSolidInstances(structuredClone(moved), cache).bodies[8]!.mesh.positions[0]).not.toBe(999)
})

it('keeps Undo/Redo and the persisted text identical on a linked scene', async () => {
  const document = await linkedScene(60)
  const history = new DirectHistory(document)
  const reference = new DirectHistory(document)
  const next = history.document
  next.bodies[3]!.instance!.matrix = placement(-40)
  history.commit(next)
  reference.commit(structuredClone(next))
  const committed = history.syncedDocument
  // Autosave reuses the snapshot text; it equals a fresh serialization of the same state.
  expect(history.canonicalText(committed)).toBe(serializeDirectDocument(committed))
  expect(history.canonicalText(history.document)).toBeUndefined()
  expect(stringifyMeshJson(history.undo())).toBe(stringifyMeshJson(reference.undo()))
  // A copy registered for an older state is no longer canonical.
  expect(history.canonicalText(committed)).toBeUndefined()
  expect(stringifyMeshJson(history.redo())).toBe(stringifyMeshJson(reference.redo()))
  const restored = await history.restoreAsync('undo', async text =>
    parseDirectDocument(text, new SolidInstanceBatchCache()),
  )
  expect(restored).toBe(true)
  expect(history.snapshotKey).toBe(serializeDirectDocument(document))
  let loaded: DirectDocument | undefined
  expect(await history.restoreAsync('redo', async text => (loaded = parseDirectDocument(text)))).toBe(true)
  expect(history.canonicalText(loaded!)).toBe(serializeDirectDocument(loaded!))
})

it('keeps the linked-instance commit policy while comparing without serialization', async () => {
  const document = await linkedScene(30)
  const resolved = parseDirectDocument(serializeDirectDocument(document))
  // Moving the placement is allowed; changing the geometry of a live instance is not.
  const moved = structuredClone(resolved)
  moved.bodies[2]!.instance!.matrix = placement(-10)
  expect(() => prepareDocumentCommit(moved, resolved, [], [], '', 'edit', label)).not.toThrow()
  const edited = structuredClone(resolved)
  ;(edited.bodies[2]!.mesh.positions as number[] | Float64Array)[0] = 123
  expect(() => prepareDocumentCommit(edited, resolved, [], [], '', 'edit', label)).toThrow('Измените источник')
  const locked = prepareDocumentCommit(structuredClone(resolved), resolved, [], ['linked-1'], '', 'edit', label)
  expect(() => locked.validateLocked!(structuredClone(resolved))).not.toThrow()
  expect(() =>
    locked.validateLocked!({ ...resolved, bodies: resolved.bodies.filter(b => b.id !== 'linked-1') }),
  ).toThrow('Объект заблокирован')
})

it('stays inside generous time bounds on 1 source + 299 linked instances', async () => {
  const document = await linkedScene(299)
  const history = new DirectHistory(document)
  const next = history.document
  next.bodies[1]!.name = 'renamed'
  let started = performance.now()
  prepareDocumentCommit(next, document, [], [], '', 'edit', label)
  const policy = performance.now() - started
  started = performance.now()
  history.commit(next)
  const commit = performance.now() - started
  const cache = new SolidInstanceBatchCache()
  parseDirectDocument(history.snapshotKey, cache)
  await history.restoreAsync('undo', async text => parseDirectDocument(text, cache))
  started = performance.now()
  await history.restoreAsync('redo', async text => structuredClone(parseDirectDocument(text, cache)))
  const redo = performance.now() - started
  // Before C14: policy ~150 ms, synchronous commit ~850 ms, cold Redo ~1.5 s (node, M4 Max).
  expect(policy).toBeLessThan(400)
  expect(commit).toBeLessThan(1500)
  expect(redo).toBeLessThan(1500)
}, 120_000)

it('derives the draft text from the snapshot text exactly like a fresh draft serialization', async () => {
  const document = await linkedScene(3)
  const fresh = (d: DirectDocument) => {
    const persisted = { ...d }
    if (!persisted.curves?.length) delete persisted.curves
    if (!persisted.surfaces?.length) delete persisted.surfaces
    return serializeDirectDocument(persisted)
  }
  const variants: DirectDocument[] = [
    document,
    { ...document, groups: [{ id: 'g', name: 'G' }] } as DirectDocument,
    { surfaces: [], version: 1, curves: [], sketches: [], bodies: [] },
    { version: 1, sketches: [], bodies: [] },
    { ...document, blenderProjectId: 'p' },
  ]
  for (const d of variants) {
    expect(serializeDirectDraft(d, serializeDirectDocument(d))).toBe(fresh(d))
    expect(serializeDirectDraft(d)).toBe(fresh(d))
  }
})
