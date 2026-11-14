import { readFileSync } from 'node:fs'
import { IDBFactory } from 'fake-indexeddb'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  emptyDirectDocument,
  serializeDirectDocument,
  type DirectBody,
  type DirectDocument,
  type DirectSketch,
} from '../src/app/directModeling'
import { applyDirectExtrusionProfile } from '../src/app/directExtrusion'
import { editSketchDimension } from '../src/app/directDimensions'
import { setLiveBodyPattern } from '../src/app/liveBodyPattern'
import { executeDirectTransaction } from '../src/app/directTransactions'
import { shellSolid, solidTopology, DirectSolidCapabilityError } from '../src/app/directSolidTools'
import { applySolidBodyEdit, type SolidBodyEditOptions } from '../src/kernel/solidBodyEdit'
import { applySolidBoolean } from '../src/kernel/solidBoolean'
import { applySolidRevolve, type SolidRevolveOptions } from '../src/kernel/solidRevolve'
import { applySolidSceneEdit, type SolidSceneEditOptions } from '../src/kernel/solidSceneEdit'
import { buildSolidSurface } from '../src/kernel/solidSurfaceConstruction'
import { prepareSolidStepImport } from '../src/kernel/solidStepExchange'
import { measureSolidMassProperties } from '../src/kernel/solidMeasurements'
import { createBrepBox, tessellateNurbsBrep } from '../src/kernel/geometry/brep'
import { evaluateNurbsSurface } from '../src/kernel/nurbsSurface'
import { warmGeometryKernel } from '../src/kernel/geometry/kernel'
import { controlFailureMessage } from './controlModelSupport'

/**
 * Twelve control models (roadmap C3). Each model is built through the same
 * document functions the direct modeler runs in its worker, checked against
 * analytic volume, surface area and bounds, and paired with at least one
 * refusal that must leave the document byte-for-byte unchanged.
 */

type Expected = { volume?: number; area?: number; bounds: [[number, number, number], [number, number, number]] }
type Refusal = {
  name: string
  run: (document: DirectDocument) => unknown
  error: RegExp
  code?: RegExp
  ru?: RegExp
  en?: RegExp
}
interface ControlModel {
  name: string
  build: () => DirectDocument | Promise<DirectDocument>
  result: string
  expected: Expected
  refusals: Refusal[]
}

const rectangle = (
  id: string,
  x: number,
  y: number,
  w: number,
  h: number,
  extra: Partial<DirectSketch> = {},
): DirectSketch => ({
  id,
  name: id,
  closed: true,
  points: [
    [x, y],
    [x + w, y],
    [x + w, y + h],
    [x, y + h],
  ],
  ...extra,
})
const circle = (
  id: string,
  cx: number,
  cy: number,
  radius: number,
  extra: Partial<DirectSketch> = {},
): DirectSketch => ({
  id,
  name: id,
  closed: true,
  points: [],
  analytic: { kind: 'circle', center: [cx, cy], radius, start: 0, sweep: 360 },
  ...extra,
})
const boxBody = (id: string, min: [number, number, number], max: [number, number, number]): DirectBody => {
  const brep = createBrepBox(min, max),
    mesh = tessellateNurbsBrep(brep, 4)
  return { id, name: id, brep, mesh: { positions: mesh.positions, indices: mesh.indices } }
}
const documentWith = (parts: { sketches?: DirectSketch[]; bodies?: DirectBody[] }): DirectDocument => ({
  ...emptyDirectDocument(),
  sketches: parts.sketches ?? [],
  bodies: parts.bodies ?? [],
})
const extrude = (
  document: DirectDocument,
  sketchIds: string[],
  height: number,
  id: string,
  operation: 'new' | 'union' | 'difference' = 'new',
  targetId = '',
) => applyDirectExtrusionProfile(document, { sketchIds, height, offset: 0, operation, targetId, id })
const revolveOptions: SolidRevolveOptions = {
  sketchId: 'profile',
  geometry: 'exact',
  operation: 'new',
  targetId: '',
  id: 'revolved',
  name: 'Revolved',
  tessellation: 4,
  axis: 'y',
  offset: 0,
  angle: 360,
  segments: 48,
}
const scene = (options: Partial<SolidSceneEditOptions>): SolidSceneEditOptions => ({
  operation: 'transform',
  id: '',
  ids: [],
  createdId: '',
  x: 0,
  y: 0,
  z: 0,
  axis: 'z',
  angle: 0,
  scale: 1,
  ...options,
})
const bodyEdit = (options: Partial<SolidBodyEditOptions>): SolidBodyEditOptions => ({
  operation: 'edge-fillet',
  id: '',
  face: 0,
  edges: [],
  openings: [],
  segments: 8,
  distance: 0,
  radius: 1,
  endRadius: 1,
  filletMode: 'constant',
  axis: 'z',
  ...options,
})
const verticalEdges = (body: DirectBody) =>
  solidTopology(body.mesh).edges.flatMap((edge, i) => {
    const p = body.mesh.positions,
      a = [p[edge.a * 3], p[edge.a * 3 + 1]],
      b = [p[edge.b * 3], p[edge.b * 3 + 1]]
    return Math.abs(a[0] - b[0]) < 1e-8 && Math.abs(a[1] - b[1]) < 1e-8 ? [i] : []
  })
const line = (id: string, from: number[], to: number[]) => ({
  id,
  name: id,
  curve: { degree: 1, knots: [0, 0, 1, 1], controlPoints: [from, to], weights: [1, 1] },
})
const sweepOptions = {
  mode: 'translation' as const,
  normal: [0, 0, 1] as [number, number, number],
  sections: 24,
  maxDeviation: 0.01,
}
const stepSource = readFileSync(
  new URL('./fixtures/step-v6/self-authored-ap242-periodic-cylinder.step', import.meta.url),
  'utf8',
)

function surfaceArea(document: DirectDocument, id: string) {
  const surface = document.surfaces!.find(s => s.id === id)!.surface,
    n = 16
  let area = 0
  for (let i = 0; i < n; i++)
    for (let j = 0; j < n; j++) {
      const u = (i + 0.5) / n,
        v = (j + 0.5) / n,
        e = evaluateNurbsSurface(surface, u, v)
      const [a, b] = [e.du!, e.dv!]
      area += Math.hypot(a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]) / n / n
    }
  return area
}
function surfaceBounds(document: DirectDocument, id: string) {
  const surface = document.surfaces!.find(s => s.id === id)!.surface,
    min = [Infinity, Infinity, Infinity],
    max = [-Infinity, -Infinity, -Infinity]
  for (const u of [0, 0.5, 1])
    for (const v of [0, 0.5, 1])
      evaluateNurbsSurface(surface, u, v).point.forEach((x, k) => {
        min[k] = Math.min(min[k], x)
        max[k] = Math.max(max[k], x)
      })
  return [min, max]
}

export const controlModels: ControlModel[] = [
  {
    name: 'plate with two through holes',
    build: () =>
      extrude(
        documentWith({
          sketches: [rectangle('plate', 0, 0, 60, 40), circle('hole-a', 15, 20, 4), circle('hole-b', 45, 20, 4)],
        }),
        ['plate', 'hole-a', 'hole-b'],
        5,
        'plate-body',
      ),
    result: 'plate-body',
    expected: {
      volume: (2400 - 32 * Math.PI) * 5,
      area: 2 * (2400 - 32 * Math.PI) + 200 * 5 + 2 * 8 * Math.PI * 5,
      bounds: [
        [0, 0, 0],
        [60, 40, 5],
      ],
    },
    refusals: [
      {
        name: 'zero extrusion height',
        run: d => extrude(d, ['plate'], 0, 'bad'),
        error: /height|distance|zero|positive/i,
        code: /^DIRECT_EXTRUDE_HEIGHT$/,
        ru: /Высота выдавливания должна быть ненулевой/,
        en: /extrusion height must be nonzero/,
      },
      {
        name: 'missing profile',
        run: d => extrude(d, ['missing'], 5, 'bad'),
        error: /Profile contour no longer exists/,
        code: /^DIRECT_PROFILE_MISSING$/,
        ru: /Контур профиля больше не существует/,
      },
    ],
  },
  {
    name: 'bracket block with filleted vertical edges',
    build: () => {
      const d = documentWith({ bodies: [boxBody('bracket', [0, 0, 0], [40, 30, 20])] })
      return applySolidBodyEdit(d, bodyEdit({ id: 'bracket', edges: verticalEdges(d.bodies[0]), radius: 3 }))
    },
    result: 'bracket',
    expected: {
      volume: 24000 - (4 - Math.PI) * 9 * 20,
      bounds: [
        [0, 0, 0],
        [40, 30, 20],
      ],
    },
    refusals: [
      {
        name: 'fillet radius larger than the part',
        run: d =>
          applySolidBodyEdit(d, bodyEdit({ id: 'bracket', edges: verticalEdges(d.bodies[0]).slice(0, 1), radius: 50 })),
        error: /./,
        code: /^BREP_[A-Z_]*FILLET_REFUSED$/,
        ru: /Уменьшите радиус|не подтверждено|рёбер/,
        en: /Reduce the radius|unproven|edge/,
      },
    ],
  },
  {
    name: 'shelled box (open-top enclosure)',
    build: () =>
      applySolidBoolean(
        documentWith({
          bodies: [boxBody('outer', [0, 0, 0], [40, 30, 20]), boxBody('cavity', [2, 2, 2], [38, 28, 22])],
        }),
        { operation: 'difference', a: ['outer'], b: ['cavity'], segments: 4 },
      ).document,
    result: 'outer',
    expected: {
      volume: 7152,
      bounds: [
        [0, 0, 0],
        [40, 30, 20],
      ],
    },
    refusals: [
      {
        name: 'mesh shell on an exact B-rep',
        run: d => shellSolid(d.bodies[0], [0], 1),
        error: /analytic-shell/,
        code: /^BREP_ANALYTIC_SHELL_REFUSED$/,
        ru: /Точная оболочка .* не подтверждена.*Переведите тело в сетку/,
        en: /exact shell is unproven.*Convert the body to a mesh/,
      },
      {
        name: 'Boolean without a tool body',
        run: d => applySolidBoolean(d, { operation: 'difference', a: ['outer'], b: [], segments: 4, ru: true }),
        error: /Заполните оба поля/,
      },
    ],
  },
  {
    name: 'revolved ring',
    build: () => applySolidRevolve(documentWith({ sketches: [rectangle('profile', 3, 0, 3, 4)] }), revolveOptions),
    result: 'revolved',
    expected: {
      volume: Math.PI * (36 - 9) * 4,
      area: 2 * Math.PI * (36 - 9) + 2 * Math.PI * (6 + 3) * 4,
      bounds: [
        [-6, 0, -6],
        [6, 4, 6],
      ],
    },
    refusals: [
      {
        name: 'zero revolve angle',
        run: d => applySolidRevolve(d, { ...revolveOptions, id: 'bad', angle: 0 }),
        error: /angle|positive|range/i,
        code: /^BREP_INVALID_ANGLE$/,
        ru: /Угол вращения/,
        en: /revolve angle/,
      },
      {
        name: 'profile crossing the axis',
        run: d =>
          applySolidRevolve({ ...d, sketches: [rectangle('profile', -2, 0, 4, 4)] }, { ...revolveOptions, id: 'bad' }),
        error: /axis|cross|side/i,
        code: /^DIRECT_REVOLVE_AXIS$/,
        ru: /пересекает ось вращения/,
      },
    ],
  },
  {
    name: 'plate with a 2 x 3 pattern of holes',
    build: () => {
      let d = documentWith({ bodies: [boxBody('plate', [0, 0, 0], [60, 40, 4])] })
      d = extrude(
        { ...d, sketches: [circle('pin', 10, 10, 2, { plane: { origin: [0, 0, -1], u: [1, 0, 0], v: [0, 1, 0] } })] },
        ['pin'],
        6,
        'pin-body',
      )
      d = setLiveBodyPattern(d, 'pin-body', { kind: 'grid', rows: 2, columns: 3, spacing: [20, 20] })
      d = applySolidSceneEdit(
        d,
        scene({ operation: 'instance-detach', ids: d.bodies.filter(b => b.instance).map(b => b.id) }),
      )
      return applySolidBoolean(d, {
        operation: 'difference',
        a: ['plate'],
        b: d.bodies.filter(b => b.id !== 'plate').map(b => b.id),
        segments: 4,
      }).document
    },
    result: 'plate',
    expected: {
      volume: 60 * 40 * 4 - 6 * Math.PI * 4 * 4,
      bounds: [
        [0, 0, 0],
        [60, 40, 4],
      ],
    },
    refusals: [
      {
        name: 'empty grid',
        run: d =>
          executeDirectTransaction(
            d,
            JSON.stringify({
              version: 1,
              actions: [
                {
                  kind: 'pattern',
                  sourceId: 'plate',
                  pattern: { kind: 'grid', rows: 0, columns: 3, spacing: [20, 20] },
                },
              ],
            }),
          ),
        error: /^Action 1: /,
      },
      {
        name: 'pattern source that does not exist',
        run: d => setLiveBodyPattern(d, 'missing', { kind: 'grid', rows: 2, columns: 2, spacing: [5, 5] }),
        error: /independent source body/,
      },
    ],
  },
  {
    name: 'swept rail surface',
    build: () => {
      const d = {
        ...emptyDirectDocument(),
        curves: [line('profile', [0, 0, 0], [0, 10, 0]), line('path', [0, 0, 0], [50, 0, 0])],
      }
      const built = buildSolidSurface(d, {
        kind: 'nurbs-sweep',
        ids: ['profile', 'path'],
        id: 'rail',
        reversed: [],
        sweep: sweepOptions,
      })
      if (!built.document) throw Error(built.error)
      return built.document
    },
    result: 'rail',
    expected: {
      area: 500,
      bounds: [
        [0, 0, 0],
        [50, 10, 0],
      ],
    },
    refusals: [
      {
        name: 'profile reused as path',
        run: d => {
          const r = buildSolidSurface(d, {
            kind: 'nurbs-sweep',
            ids: ['profile', 'profile'],
            id: 'bad',
            reversed: [],
            sweep: sweepOptions,
          })
          if (!r.document) throw Error(r.error)
        },
        error: /distinct/,
      },
      {
        name: 'missing path',
        run: d => {
          const r = buildSolidSurface(d, {
            kind: 'nurbs-sweep',
            ids: ['profile', 'missing'],
            id: 'bad',
            reversed: [],
            sweep: sweepOptions,
          })
          if (!r.document) throw Error(r.error)
        },
        error: /Curve missing/,
      },
    ],
  },
  {
    name: 'lofted square transition',
    build: () =>
      applySolidSceneEdit(
        documentWith({
          sketches: [
            rectangle('base', 0, 0, 10, 10),
            rectangle('top', 2, 2, 6, 6, { plane: { origin: [0, 0, 10], u: [1, 0, 0], v: [0, 1, 0] } }),
          ],
        }),
        scene({ operation: 'loft', id: 'base', ids: ['base', 'top'], createdId: 'loft' }),
      ),
    result: 'loft',
    expected: {
      volume: (10 / 3) * (100 + 36 + 60),
      bounds: [
        [0, 0, 0],
        [10, 10, 10],
      ],
    },
    refusals: [
      {
        name: 'single section',
        run: d => applySolidSceneEdit(d, scene({ operation: 'loft', id: 'base', ids: ['base'], createdId: 'bad' })),
        error: /./,
        code: /^DIRECT_LOFT_SECTIONS$/,
        ru: /от 2 до 64 замкнутых сечений/,
        en: /2 to 64 closed sections/,
      },
    ],
  },
  {
    name: 'Boolean union chain',
    build: () => {
      let d = documentWith({
        bodies: [
          boxBody('a', [0, 0, 0], [10, 10, 10]),
          boxBody('b', [5, 0, 0], [15, 10, 10]),
          boxBody('c', [12, 0, 0], [20, 10, 4]),
        ],
      })
      d = applySolidBoolean(d, { operation: 'union', a: ['a'], b: ['b'], segments: 4 }).document
      return applySolidBoolean(d, { operation: 'union', a: ['a'], b: ['c'], segments: 4 }).document
    },
    result: 'a',
    expected: {
      volume: 1500 + 5 * 10 * 4,
      bounds: [
        [0, 0, 0],
        [20, 10, 10],
      ],
    },
    refusals: [
      {
        name: 'union without operands (English)',
        run: d => applySolidBoolean(d, { operation: 'union', a: [], b: ['a'], segments: 4 }),
        error: /Fill both fields/,
      },
    ],
  },
  {
    name: 'Boolean subtract chain',
    build: () => {
      let d = documentWith({
        bodies: [
          boxBody('block', [0, 0, 0], [30, 20, 10]),
          boxBody('slot', [5, -1, 6], [25, 21, 11]),
          boxBody('notch', [-1, 8, -1], [4, 12, 11]),
        ],
      })
      d = applySolidBoolean(d, { operation: 'difference', a: ['block'], b: ['slot'], segments: 4 }).document
      return applySolidBoolean(d, { operation: 'difference', a: ['block'], b: ['notch'], segments: 4 }).document
    },
    result: 'block',
    expected: {
      volume: 6000 - 20 * 20 * 4 - 4 * 4 * 10,
      bounds: [
        [0, 0, 0],
        [30, 20, 10],
      ],
    },
    refusals: [
      {
        name: 'subtract a body from itself',
        run: d => applySolidBoolean(d, { operation: 'difference', a: ['block'], b: ['block'], segments: 4 }),
        error: /./,
      },
    ],
  },
  {
    name: 'imported STEP body',
    build: async () => (await prepareSolidStepImport(stepSource, emptyDirectDocument())).document,
    result: '',
    expected: {
      volume: 12 * Math.PI,
      area: 20 * Math.PI,
      bounds: [
        [-2, -2, 0],
        [2, 2, 3],
      ],
    },
    refusals: [
      {
        name: 'malformed STEP text',
        run: d =>
          prepareSolidStepImport('ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1=FOO();\nENDSEC;\nEND-ISO-10303-21;\n', d),
        error: /./,
        code: /^STEP_UNSUPPORTED_ROOT$/,
        ru: /В STEP-файле нет поддерживаемого/,
        en: /STEP file has no supported/,
      },
    ],
  },
  {
    name: 'dimension-driven cylinder profile',
    build: () => {
      const sketch = editSketchDimension(circle('disc', 0, 0, 5, { dimensions: [{ kind: 'diameter' }] }), 0, 14)
      return extrude(documentWith({ sketches: [sketch] }), ['disc'], 8, 'disc-body')
    },
    result: 'disc-body',
    expected: {
      volume: Math.PI * 49 * 8,
      area: 2 * Math.PI * 49 + 2 * Math.PI * 7 * 8,
      bounds: [
        [-7, -7, 0],
        [7, 7, 8],
      ],
    },
    refusals: [
      {
        name: 'negative diameter',
        run: d => editSketchDimension(d.sketches[0], 0, -2),
        error: /positive/i,
        code: /^DIRECT_DIMENSION_POSITIVE$/,
        ru: /Размер должен быть положительным/,
      },
      {
        name: 'self-intersecting profile',
        run: d =>
          extrude(
            {
              ...d,
              sketches: [
                ...d.sketches,
                {
                  id: 'bow',
                  name: 'bow',
                  closed: true,
                  points: [
                    [0, 0],
                    [10, 10],
                    [10, 0],
                    [0, 10],
                  ],
                },
              ],
            },
            ['bow'],
            3,
            'bad',
          ),
        error: /./,
        code: /^DIRECT_SKETCH_SELF_INTERSECTION$/,
        ru: /Контур эскиза пересекает сам себя.*Переставьте вершины/,
        en: /sketch contour intersects itself/,
      },
    ],
  },
  {
    name: 'chamfered block',
    build: () => {
      const d = documentWith({ bodies: [boxBody('block', [0, 0, 0], [10, 8, 6])] })
      return applySolidBodyEdit(
        d,
        bodyEdit({ operation: 'chamfer', id: 'block', edges: verticalEdges(d.bodies[0]).slice(0, 1), radius: 1 }),
      )
    },
    result: 'block',
    expected: {
      volume: 477,
      bounds: [
        [0, 0, 0],
        [10, 8, 6],
      ],
    },
    refusals: [
      {
        name: 'zero chamfer size',
        run: d =>
          applySolidBodyEdit(
            d,
            bodyEdit({ operation: 'chamfer', id: 'block', edges: verticalEdges(d.bodies[0]).slice(0, 1), radius: 0 }),
          ),
        error: /./,
      },
    ],
  },
]

beforeAll(async () => {
  await warmGeometryKernel()
})
beforeEach(() => vi.stubGlobal('indexedDB', new IDBFactory()))
afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

describe('C3 control models', () => {
  it('defines twelve models, each with a refusal', () => {
    expect(controlModels).toHaveLength(12)
    expect(controlModels.every(m => m.refusals.length > 0)).toBe(true)
  })
  it.each(controlModels.map(m => [m.name, m] as const))(
    '%s',
    async (_name, model) => {
      const document = await model.build()
      if (model.name === 'swept rail surface') {
        expect(surfaceArea(document, model.result)).toBeCloseTo(model.expected.area!, 6)
        const [min, max] = surfaceBounds(document, model.result)
        min.forEach((x, k) => expect(x).toBeCloseTo(model.expected.bounds[0][k], 6))
        max.forEach((x, k) => expect(x).toBeCloseTo(model.expected.bounds[1][k], 6))
      } else {
        const body = model.result ? document.bodies.find(b => b.id === model.result) : document.bodies[0]
        expect(body?.brep, 'exact B-rep result').toBeDefined()
        const mass = measureSolidMassProperties(body!)
        if (model.expected.volume !== undefined)
          expect(Math.abs(mass.volumeMm3 - model.expected.volume)).toBeLessThan(model.expected.volume * 1e-6)
        if (model.expected.area !== undefined)
          expect(Math.abs(mass.surfaceAreaMm2 - model.expected.area)).toBeLessThan(model.expected.area * 1e-6)
        for (let k = 0; k < 3; k++) {
          expect(mass.bounds.min[k]).toBeCloseTo(model.expected.bounds[0][k], 5)
          expect(mass.bounds.max[k]).toBeCloseTo(model.expected.bounds[1][k], 5)
        }
      }
      for (const refusal of model.refusals) {
        const before = serializeDirectDocument(document)
        let caught: unknown
        try {
          await refusal.run(document)
        } catch (error) {
          caught = error
        }
        expect(caught, refusal.name + ' must be refused').toBeInstanceOf(Error)
        expect((caught as Error).message, refusal.name).toMatch(refusal.error)
        if (refusal.code) expect((caught as { code?: string }).code, refusal.name).toMatch(refusal.code)
        if (refusal.ru) expect(controlFailureMessage(caught, 'ru'), refusal.name).toMatch(refusal.ru)
        if (refusal.en) expect(controlFailureMessage(caught, 'en'), refusal.name).toMatch(refusal.en)
        expect(serializeDirectDocument(document), refusal.name + ' left the document unchanged').toBe(before)
      }
    },
    120_000,
  )
  it('reports exact B-rep shell refusals with a typed capability error', () => {
    const body = boxBody('box', [0, 0, 0], [4, 4, 4])
    expect(() => shellSolid(body, [0], 1)).toThrow(DirectSolidCapabilityError)
  })
})
