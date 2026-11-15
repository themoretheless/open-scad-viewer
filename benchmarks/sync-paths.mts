/**
 * C1 audit: node-side p50/p95 of the main-thread synchronous paths of the direct modeler.
 *
 *   node --import tsx --import ./src/mcp/registerRawSourceImports.mjs benchmarks/sync-paths.mts [out.json] [--quick]
 *
 * Each row times one call of the function the UI runs synchronously on the main thread
 * (see the inventory in docs/design/cad-remaining-2026-09-29.md, «Аудит синхронных путей»).
 * These are V8/node timings of the JavaScript and WASM work only; browser frame time,
 * layout, GC pauses caused by other work and postMessage delivery are not included.
 */
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import os from 'node:os'
import {
  DirectHistory,
  emptyDirectDocument,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectBody,
  type DirectDocument,
} from '../src/app/directModeling'
import { stringifyMeshJson } from '../src/exchange/meshJson'
import { prepareDocumentCommit } from '../src/features/directModeler/prepareDocumentCommit'
import {
  applyDirectDocumentDelta,
  directDocumentDelta,
  directSnapshotDelta,
  isDirectDocumentDelta,
} from '../src/kernel/directDocumentDelta'
import { resolveSolidInstances } from '../src/kernel/solidInstances'
import { SolidInstanceBatchCache } from '../src/kernel/solidInstanceBatchCache'
import { setLiveBodyPattern } from '../src/app/liveBodyPattern'
import { applySolidBoolean } from '../src/kernel/solidBoolean'
import { resolveModelingSnap, type SnapGeometry } from '../src/app/modelingSnaps'
import { createDirectProjector, defaultDirectCamera } from '../src/app/directModelingTools'
import { pickDirectMesh } from '../src/app/directPicking'
import { cadRoadmapParts } from './cad-roadmap-fixtures'
import { transformNurbsBrep } from '../src/kernel/geometry/brep'
import { transformPolygonMesh } from '../src/kernel/geometry/polygon'

const quick = process.argv.includes('--quick')
const outArg = process.argv.slice(2).find(arg => !arg.startsWith('--'))
const WARMUP = quick ? 1 : 3
const SAMPLES = quick ? 5 : 20
export const TARGETS = { visual: 50, preview: 150, cancel: 100 } as const
type Target = keyof typeof TARGETS

const parts = cadRoadmapParts().map(part => part.body)
const translate = (body: DirectBody, id: string, dx: number, dy: number): DirectBody => {
  const matrix = [
    [1, 0, 0, dx],
    [0, 1, 0, dy],
    [0, 0, 1, 0],
    [0, 0, 0, 1],
  ]
  const mesh = transformPolygonMesh(body.mesh, matrix)
  return {
    id,
    name: id,
    brep: transformNurbsBrep(body.brep!, matrix),
    mesh: { positions: Array.from(mesh.positions), indices: Array.from(mesh.indices) },
  }
}
function independentDocument(count: number): DirectDocument {
  const document = emptyDirectDocument()
  for (let i = 0; i < count; i++)
    document.bodies.push(translate(parts[i % parts.length]!, `body-${i}`, (i % 20) * 50, Math.floor(i / 20) * 50))
  return parseDirectDocument(stringifyMeshJson(document))
}
function linkedDocument(instances: number): DirectDocument {
  const document = emptyDirectDocument()
  // Linked instances expand to full copies; the simple box keeps 1000 of them within the 64 MB bound.
  document.bodies.push(translate(parts[3]!, 'source', 0, -80))
  for (let i = 0; i < instances; i++) {
    const sourceId = 'source'
    document.bodies.push({
      id: `instance-${i}`,
      name: `instance-${i}`,
      mesh: { positions: [], indices: [] },
      instance: {
        sourceId,
        matrix: [
          [1, 0, 0, (i % 40) * 50],
          [0, 1, 0, Math.floor(i / 40) * 50],
          [0, 0, 1, 0],
          [0, 0, 0, 1],
        ],
      },
    })
  }
  return parseDirectDocument(serializeDirectDocument(document))
}

const scenes: Array<{ name: string; bodies: number; document: () => DirectDocument }> = [
  { name: 'small: 10 independent bodies', bodies: 10, document: () => independentDocument(10) },
  { name: 'medium: 100 independent bodies', bodies: 100, document: () => independentDocument(100) },
  { name: 'large: 200 independent bodies (document limit)', bodies: 200, document: () => independentDocument(200) },
  { name: 'large: 1 source + 999 linked instances', bodies: 1000, document: () => linkedDocument(999) },
]

const quantile = (sorted: number[], q: number) => sorted[Math.min(sorted.length - 1, Math.ceil(q * sorted.length) - 1)]!
const gc = (globalThis as { gc?: () => void }).gc
async function time(run: () => unknown | Promise<unknown>, setup?: () => unknown | Promise<unknown>) {
  const samples: number[] = []
  // Peak memory: the largest heap growth observed across one call (heapUsed after minus before,
  // with a forced GC before the call when node runs with --expose-gc). Retained results count.
  let peakHeap = 0
  for (let i = -WARMUP; i < SAMPLES; i++) {
    await setup?.()
    gc?.()
    const before = process.memoryUsage().heapUsed
    const start = performance.now()
    const result = await run()
    const elapsed = performance.now() - start
    const grown = process.memoryUsage().heapUsed - before
    void result
    if (i >= 0) {
      samples.push(elapsed)
      peakHeap = Math.max(peakHeap, grown)
    }
  }
  samples.sort((a, b) => a - b)
  return { p50: quantile(samples, 0.5), p95: quantile(samples, 0.95), peakHeapMb: peakHeap / 1048576 }
}

type Row = {
  path: string
  site: string
  target: Target
  scene: string
  p50: number
  p95: number
  peakHeapMb: number
  overTarget: boolean
}
const rows: Row[] = []
async function measure(
  scene: string,
  path: string,
  site: string,
  target: Target,
  run: () => unknown | Promise<unknown>,
  setup?: () => unknown | Promise<unknown>,
) {
  const { p50, p95, peakHeapMb } = await time(run, setup)
  rows.push({ path, site, target, scene, p50, p95, peakHeapMb, overTarget: p95 > TARGETS[target] })
}

const sceneFilter = process.argv.find(arg => arg.startsWith('--scene='))?.slice('--scene='.length)
for (const scene of scenes.filter(scene => !sceneFilter || scene.name.includes(sceneFilter))) {
  const document = scene.document()
  const name = scene.name
  const serialized = serializeDirectDocument(document)
  const renamed = () => {
    const next = structuredClone(document)
    next.bodies[0]!.name = `renamed-${Math.random()}`
    return next
  }

  // Document copy and serialization.
  await measure(
    name,
    'copy: structuredClone (history.document before C14)',
    'DirectHistory.document; gesture start, sync()',
    'visual',
    () => structuredClone(document),
  )
  const copyHistory = new DirectHistory(document)
  await measure(
    name,
    'copy: history.document (cloneDocumentTree)',
    'DirectHistory.document; sync() after a synchronous commit, mutating callers',
    'visual',
    () => copyHistory.document,
  )
  const synced = copyHistory.syncedDocument
  await measure(
    name,
    'autosave text: history snapshot reuse (canonicalText)',
    'useDraftPersistence.persist after commit/restore',
    'visual',
    () => {
      const text = copyHistory.canonicalText(synced)
      if (text === undefined) throw Error('synced document is not canonical')
      return text
    },
  )
  await measure(
    name,
    'serialize: serializeDirectDocument',
    'useDraftPersistence.persist; directTransformWorker payload',
    'visual',
    () => serializeDirectDocument(document),
  )
  await measure(
    name,
    'parse: parseDirectDocument (validate + resolve)',
    'DirectHistory.commit; import fallback',
    'preview',
    () => parseDirectDocument(serialized),
  )

  // History commit.
  let history = new DirectHistory(document)
  let next = renamed()
  await measure(
    name,
    'history commit, synchronous',
    'DirectModeler commit() -> DirectHistory.commit',
    'preview',
    () => history.commit(next),
    () => {
      history = new DirectHistory(document)
      next = renamed()
    },
  )
  let validated = parseDirectDocument(stringifyMeshJson(renamed()))
  await measure(
    name,
    'history commit, worker-validated (commitAsync)',
    'commitAsync after worker parse',
    'visual',
    () => history.commitAsync(async () => validated),
    () => {
      history = new DirectHistory(document)
      validated = parseDirectDocument(stringifyMeshJson(renamed()))
    },
  )
  await measure(name, 'commit policy: prepareDocumentCommit', 'DirectModeler prepareCommit', 'visual', () =>
    prepareDocumentCommit(next, document, [], [], '', 'edit', (ru: string) => ru),
  )
  await measure(
    name,
    'commit policy: prepareDocumentCommit, worker result (C1)',
    'commitWorkerResult/gizmo -> prepareCommit(resolved)',
    'visual',
    () => prepareDocumentCommit(next, document, [], [], '', 'edit', (ru: string) => ru, true),
  )
  // C1: a worker result that shares the published document's unchanged objects (a delta
  // answer rebuilt on the client) commits by reusing the history's own copies of them.
  let sharedHistory = new DirectHistory(document)
  let shared = document
  await measure(
    name,
    'history commit, worker result sharing unchanged objects (C1)',
    'gizmo/transform delta -> commitAsync',
    'visual',
    () => sharedHistory.commitAsync(async () => shared),
    () => {
      sharedHistory = new DirectHistory(document)
      const published = sharedHistory.syncedDocument
      const renamedBody = { ...published.bodies[0]!, name: `renamed-${Math.random()}` }
      shared = { ...published, bodies: [renamedBody, ...published.bodies.slice(1)] }
    },
  )

  // Undo/Redo: the worker parses the snapshot text (restoreDocument with its instance cache),
  // the response crosses postMessage (structuredClone proxy), the main thread applies it.
  const workerCache = new SolidInstanceBatchCache()
  const restoreInWorker = async (text: string) => structuredClone(parseDirectDocument(text, workerCache))
  let redoHistory = new DirectHistory(document)
  await measure(
    name,
    'history Redo: worker restore + response clone + apply',
    'DirectModeler undo(true) -> restoreAsync(restoreDocument)',
    'preview',
    async () => {
      if (!(await redoHistory.restoreAsync('redo', restoreInWorker))) throw Error('Redo did not apply')
    },
    async () => {
      redoHistory = new DirectHistory(document)
      redoHistory.commit(renamed())
      await redoHistory.restoreAsync('undo', async text => parseDirectDocument(text, workerCache))
    },
  )
  let undoHistory = new DirectHistory(document)
  await measure(
    name,
    'history Undo: worker restore + response clone + apply',
    'DirectModeler undo() -> restoreAsync(restoreDocument)',
    'preview',
    async () => {
      if (!(await undoHistory.restoreAsync('undo', restoreInWorker))) throw Error('Undo did not apply')
    },
    () => {
      undoHistory = new DirectHistory(document)
      undoHistory.commit(renamed())
    },
  )
  // C1/C14: restore answered as a delta against the published state (`restoreDocument` with
  // `base`): the worker parses the snapshot and diffs snapshot texts, only changed objects cross
  // postMessage, the client rebuilds the document from its published objects.
  const restoreDelta =
    (history: DirectHistory, published: () => DirectDocument, keep: (d: DirectDocument) => void) =>
    async (text: string) => {
      const base = published(),
        baseText = history.canonicalText(base)
      if (baseText === undefined) throw Error('published document is not canonical')
      const next = parseDirectDocument(text, workerCache)
      const wire = structuredClone(directSnapshotDelta(baseText, text, next) ?? next)
      const rebuilt = isDirectDocumentDelta(wire) ? applyDirectDocumentDelta(base, wire) : wire
      if (!rebuilt) throw Error('delta did not apply')
      keep(rebuilt)
      return rebuilt
    }
  let deltaRedoHistory = new DirectHistory(document),
    deltaRedoPublished = deltaRedoHistory.syncedDocument
  await measure(
    name,
    'history Redo: worker restore answered as delta + apply (C1/C14)',
    'DirectModeler undo(true) -> restoreDocument{base}',
    'preview',
    async () => {
      const load = restoreDelta(
        deltaRedoHistory,
        () => deltaRedoPublished,
        d => (deltaRedoPublished = d),
      )
      if (!(await deltaRedoHistory.restoreAsync('redo', load))) throw Error('Redo did not apply')
    },
    async () => {
      deltaRedoHistory = new DirectHistory(document)
      deltaRedoHistory.commit(renamed())
      deltaRedoPublished = deltaRedoHistory.syncedDocument
      const load = restoreDelta(
        deltaRedoHistory,
        () => deltaRedoPublished,
        d => (deltaRedoPublished = d),
      )
      await deltaRedoHistory.restoreAsync('undo', load)
    },
  )
  let deltaUndoHistory = new DirectHistory(document),
    deltaUndoPublished = deltaUndoHistory.syncedDocument
  await measure(
    name,
    'history Undo: worker restore answered as delta + apply (C1/C14)',
    'DirectModeler undo() -> restoreDocument{base}',
    'preview',
    async () => {
      const load = restoreDelta(
        deltaUndoHistory,
        () => deltaUndoPublished,
        d => (deltaUndoPublished = d),
      )
      if (!(await deltaUndoHistory.restoreAsync('undo', load))) throw Error('Undo did not apply')
    },
    () => {
      deltaUndoHistory = new DirectHistory(document)
      deltaUndoHistory.commit(renamed())
      deltaUndoPublished = deltaUndoHistory.syncedDocument
    },
  )
  // The first synchronous commit after an async restore adopts the restored document instead
  // of re-parsing the snapshot on the main thread.
  let afterUndoHistory = new DirectHistory(document),
    afterUndoNext = document
  await measure(
    name,
    'history commit, synchronous, right after async Undo',
    'DirectModeler commit() after undo()',
    'preview',
    () => afterUndoHistory.commit(afterUndoNext),
    async () => {
      afterUndoHistory = new DirectHistory(document)
      afterUndoHistory.commit(renamed())
      let published = afterUndoHistory.syncedDocument
      await afterUndoHistory.restoreAsync(
        'undo',
        restoreDelta(
          afterUndoHistory,
          () => published,
          d => (published = d),
        ),
      )
      afterUndoNext = {
        ...published,
        bodies: [{ ...published.bodies[0]!, name: `again-${Math.random()}` }, ...published.bodies.slice(1)],
      }
    },
  )
  const coldText = serializeDirectDocument(renamed())
  await measure(
    name,
    'history Redo, worker part, cold instance cache',
    'mainSolidWorkerRuntime restoreDocument (first visit)',
    'preview',
    () => parseDirectDocument(coldText, new SolidInstanceBatchCache()),
  )

  // Reference resolution.
  await measure(
    name,
    'reference resolution: resolveSolidInstances',
    'parseDirectDocument, transformSolidInstance',
    'preview',
    () => resolveSolidInstances(structuredClone(document)),
  )

  // Direct Boolean: the worker computes, the main thread applies the result.
  const booleanResult = applySolidBoolean(document, {
    operation: 'union',
    a: [document.bodies[0]!.id],
    b: [document.bodies[1]!.id],
    segments: 24,
    ru: false,
  })
  await measure(
    name,
    'direct Boolean apply: sync commit of worker result (before C1)',
    'useBooleanOperations runBoolean -> commit()',
    'preview',
    () => history.commit(booleanResult.document),
    () => {
      history = new DirectHistory(document)
    },
  )
  let booleanValidated = parseDirectDocument(serializeDirectDocument(booleanResult.document))
  await measure(
    name,
    'direct Boolean apply: commitAsync of validated result (C1)',
    'useBooleanOperations runBoolean -> commitWorkerResult()',
    'visual',
    () => history.commitAsync(async () => booleanValidated),
    () => {
      history = new DirectHistory(document)
      booleanValidated = parseDirectDocument(serializeDirectDocument(booleanResult.document))
    },
  )
  // C1: the worker answers the Boolean as a delta against the sent document; the client
  // rebuilds it from its published copy, so unchanged bodies are the history's own twins.
  const booleanDelta = directDocumentDelta(document, booleanValidated)
  let deltaHistory = new DirectHistory(document)
  let deltaPublished = deltaHistory.syncedDocument
  await measure(
    name,
    'direct Boolean apply: delta answer -> commitAsync (C1)',
    'useBooleanOperations runBoolean(documentDelta) -> commitWorkerResult()',
    'visual',
    () => deltaHistory.commitAsync(async () => applyDirectDocumentDelta(deltaPublished, booleanDelta)!),
    () => {
      deltaHistory = new DirectHistory(document)
      deltaPublished = deltaHistory.syncedDocument
    },
  )

  // Arrays: live pattern edit and its expansion into linked instances.
  const source = document.bodies.find(body => !body.instance)!.id
  // A 10x10 pattern adds 99 linked instances; the 1000-instance scene has no headroom left.
  if (document.bodies.filter(body => body.instance).length + 99 <= 1000)
    await measure(
      name,
      'array: setLiveBodyPattern 10x10 + resolve',
      'pattern action -> resolveSolidInstances',
      'preview',
      () =>
        resolveSolidInstances(
          setLiveBodyPattern(document, source, { kind: 'grid', rows: 10, columns: 10, spacing: [60, 60] }),
        ),
    )

  // Manipulators: per pointer move the main thread structured-clones the document into the
  // gizmo worker request and resolves snapping; the kernel edit runs in the worker.
  const snapGeometry: SnapGeometry = { points: [{ point: [0, 0, 0], kind: 'origin' }], segments: [] }
  for (const body of document.bodies)
    for (const edge of body.brep?.edges ?? []) {
      const points = edge.curve.controlPoints
      for (let i = 1; i < points.length; i++)
        snapGeometry.segments.push({
          a: points[i - 1] as [number, number, number],
          b: points[i] as [number, number, number],
        })
    }
  const projector = createDirectProjector(defaultDirectCamera())
  await measure(
    name,
    'manipulator move: snap resolve over scene edges',
    'DirectModeler snapped3 -> resolveModelingSnap',
    'visual',
    () =>
      resolveModelingSnap([123.4, 56.7, 8.9], snapGeometry, {
        project: p => projector(p).slice(0, 2) as [number, number],
        radius: 12,
        grid: 1,
        geometry: true,
        guides: true,
      }),
  )
  await measure(
    name,
    'manipulator move: worker request clone (postMessage proxy, before C1)',
    'requestDragJob -> gizmoWorker.run (whole document)',
    'visual',
    () => structuredClone({ kind: 'sceneEdit', document }),
  )
  // C1: the drag base is resident in the worker. The first move sends the snapshot text,
  // later moves only its key; the answer is a delta that the client applies to its base.
  const residentText = new DirectHistory(document).snapshotKey
  await measure(
    name,
    'manipulator first move: resident base as snapshot text (C1)',
    'gizmoWorker.run residentDocument (first request)',
    'visual',
    () => structuredClone({ kind: 'sceneEdit', document: residentText, retainAs: 'k', delta: true }),
  )
  await measure(
    name,
    'manipulator move: resident key request clone (C1)',
    'gizmoWorker.run residentDocument (later moves)',
    'visual',
    () =>
      structuredClone({
        kind: 'sceneEdit',
        document: { resident: 'k' },
        delta: true,
        options: { operation: 'transform', x: 1 },
      }),
  )
  const movedOne = { ...document, bodies: [{ ...document.bodies[0]!, name: 'moved' }, ...document.bodies.slice(1)] }
  await measure(
    name,
    'manipulator move: delta answer (worker diff + clone + client apply) (C1)',
    'mainSolidWorkerRuntime residentResult -> client applyDirectDocumentDelta',
    'visual',
    () => applyDirectDocumentDelta(document, structuredClone(directDocumentDelta(document, movedOne))),
  )

  // Screen projection: CPU ray pick (hover, GPU click) and CPU projection of all triangles.
  const camera = defaultDirectCamera()
  const candidates = document.bodies.map(body => ({ id: body.id, mesh: body.mesh }))
  await measure(
    name,
    'screen projection: ray pick over all triangles',
    'DirectModeler pickBodyAt (hover/GPU click)',
    'visual',
    () => pickDirectMesh([25, 10], camera, candidates),
  )
  await measure(
    name,
    'screen projection: project + depth-sort all triangles (CPU fallback)',
    'useViewportLayerAndPolygons polygonView',
    'visual',
    () => {
      const out: Array<{ depth: number; vertices: number[][] }> = []
      for (const body of document.bodies) {
        const { positions, indices } = body.mesh
        for (let t = 0; t < indices.length; t += 3) {
          const vertices = [indices[t]!, indices[t + 1]!, indices[t + 2]!].map(index =>
            projector([positions[index * 3]!, positions[index * 3 + 1]!, positions[index * 3 + 2]!]),
          )
          out.push({ depth: (vertices[0]![2] + vertices[1]![2] + vertices[2]![2]) / 3, vertices })
        }
      }
      return out.sort((a, b) => a.depth - b.depth)
    },
  )
  console.error(`measured ${name}`)
}

const report = {
  node: process.version,
  platform: process.platform,
  arch: process.arch,
  cpu: os.cpus()[0]?.model,
  warmups: WARMUP,
  samples: SAMPLES,
  targetsMs: TARGETS,
  scope: 'Node-side timings of main-thread synchronous paths; not browser frame time.',
  rows,
}
const out = resolve(outArg ?? 'tmp/performance/sync-paths.json')
mkdirSync(dirname(out), { recursive: true })
writeFileSync(out, JSON.stringify(report, null, 2))
const fmt = (n: number) => n.toFixed(n < 10 ? 2 : 1)
console.log('| Scene | Path | Target | p50 ms | p95 ms | peak heap MB | Over |')
console.log('|---|---|---|---:|---:|---:|---|')
for (const row of rows)
  console.log(
    `| ${row.scene} | ${row.path} | ${row.target} ≤${TARGETS[row.target]} | ${fmt(row.p50)} | ${fmt(row.p95)} | ${fmt(row.peakHeapMb)} | ${row.overTarget ? 'yes' : ''} |`,
  )
