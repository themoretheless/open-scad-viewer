import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const chart = readFileSync('examples/rush/exact-periodic-window.r', 'utf8').split('patch =')[0]!
const cases: [string, string, number, number, string?][] = [
  ['rotate_periodic_surface_at_exact_knot', 'direction:"u",parameter:1', 1.25, 0.5],
  ['insert_and_rotate_periodic_surface_seam', 'direction:"u",parameter:0.5', 0.75, 0.5],
  ['open_exact_periodic_surface_axis', 'direction:"v"', 0.75, 0.5],
  ['restrict_exact_periodic_surface_axis', 'direction:"u",interval:[3.5,4.5]', 4.25, 0.5],
  ['restrict_exact_periodic_surface_over_turns', 'direction:"u",interval:[0.5,6.5]', 5.25, 0.5],
  ['restrict_exact_doubly_periodic_surface', 'u:[3.5,4.5],v:[3.5,4.5]', 4.25, 4.25],
  ['restrict_exact_doubly_periodic_surface_over_turns', 'u:[0.5,5.5],v:[0.5,5.5]', 4.25, 4.25],
  ['partition_exact_periodic_surface_axis', 'direction:"u",seam:0,cuts:[1,2],part:1', 1.75, 0.5],
  ['partition_exact_periodic_surface_over_turns', 'direction:"v",interval:[0,6],cuts:[1,4],part:2', 0.5, 5.25],
  ['partition_exact_doubly_periodic_surface', 'seams:[0,0],u_cuts:[1],v_cuts:[2],u_index:1,v_index:1', 1.75, 2.75],
  [
    'partition_exact_doubly_periodic_surface_over_turns',
    'u:[0,6],v:[0,6],u_cuts:[4],v_cuts:[2],u_index:1,v_index:1',
    5.25,
    2.75,
  ],
  [
    'remove_exact_periodic_surface_knot',
    'direction:"u",knot:4.5',
    5.25,
    0.5,
    'insert_and_rotate_periodic_surface_seam(direction:"u",parameter:0.5,max_work:1000000).rotate_periodic_surface_at_exact_knot(direction:"u",parameter:4,max_work:1000000)',
  ],
  [
    'simplify_exact_periodic_surface_axis',
    'direction:"u"',
    5.25,
    0.5,
    'insert_and_rotate_periodic_surface_seam(direction:"u",parameter:0.5,max_work:1000000).rotate_periodic_surface_at_exact_knot(direction:"u",parameter:4,max_work:1000000)',
  ],
  [
    'remove_exact_periodic_surface_seam_knot',
    'direction:"v",anchor:1',
    1.25,
    1.5,
    'insert_and_rotate_periodic_surface_seam(direction:"v",parameter:0.5,max_work:1000000)',
  ],
  [
    'simplify_exact_doubly_periodic_surface',
    '',
    5.25,
    4.5,
    'insert_and_rotate_periodic_surface_seam(direction:"u",parameter:0.5,max_work:1000000).rotate_periodic_surface_at_exact_knot(direction:"u",parameter:4,max_work:1000000).insert_and_rotate_periodic_surface_seam(direction:"v",parameter:0.5,max_work:1000000).rotate_periodic_surface_at_exact_knot(direction:"v",parameter:4,max_work:1000000)',
  ],
  [
    'reduce_exact_periodic_surface_period',
    'direction:"v"',
    1.25,
    0.5,
    'repeat_exact_periodic_surface_period(direction:"v",turns:2,max_work:1000000)',
  ],
  ['repeat_exact_periodic_surface_period', 'direction:"u",turns:2', 5.25, 0.5],
]

it.each(cases)('preserves wrapped coordinates with %s', async (op, args, u, v, prefix) => {
  const setup = prefix ? prefix + '.' : ''
  const parameters = [args, 'max_work:1000000'].filter(Boolean).join(',')
  const source = chart + 'patch = chart.' + setup + op + '(' + parameters + ')\nshow patch.tessellate(8,8)'
  const { document } = compileRushFrontend(source)
  const surface = document.nodes[0]!.id
  const patch = document.nodes.findLast(n => n.op === op)!.id
  const result = await buildOwnNurbsAsync(document, {
    action: 'evaluate',
    evaluations: [
      { node: surface, u: u % 4, v: v % 4 },
      { node: patch, u, v },
    ],
  })
  expect(result.evaluations[1]!.point).toEqual(result.evaluations[0]!.point)
  const definition = result.report.definitions[patch] as {
    degreeU: number
    degreeV: number
    knotsU: number[]
    knotsV: number[]
    periodicU: boolean
    periodicV: boolean
  }
  const domain = (knots: number[], degree: number) => [knots[degree], knots[knots.length - degree - 1]]
  const domains = [domain(definition.knotsU, definition.degreeU), domain(definition.knotsV, definition.degreeV)]
  const expectedDomains: Record<string, number[][]> = {
    rotate_periodic_surface_at_exact_knot: [
      [1, 5],
      [0, 4],
    ],
    insert_and_rotate_periodic_surface_seam: [
      [0.5, 4.5],
      [0, 4],
    ],
    restrict_exact_periodic_surface_axis: [
      [3.5, 4.5],
      [0, 4],
    ],
    restrict_exact_periodic_surface_over_turns: [
      [0.5, 6.5],
      [0, 4],
    ],
    restrict_exact_doubly_periodic_surface: [
      [3.5, 4.5],
      [3.5, 4.5],
    ],
    restrict_exact_doubly_periodic_surface_over_turns: [
      [0.5, 5.5],
      [0.5, 5.5],
    ],
    partition_exact_periodic_surface_axis: [
      [1, 2],
      [0, 4],
    ],
    partition_exact_periodic_surface_over_turns: [
      [0, 4],
      [4, 6],
    ],
    partition_exact_doubly_periodic_surface: [
      [1, 4],
      [2, 4],
    ],
    partition_exact_doubly_periodic_surface_over_turns: [
      [4, 6],
      [2, 6],
    ],
    repeat_exact_periodic_surface_period: [
      [0, 8],
      [0, 4],
    ],
    reduce_exact_periodic_surface_period: [
      [0, 4],
      [0, 4],
    ],
  }
  if (expectedDomains[op]) expect(domains).toEqual(expectedDomains[op])
  if (op.startsWith('simplify_exact_')) expect(definition.knotsU).not.toContain(4.5)
  if (op === 'simplify_exact_doubly_periodic_surface') expect(definition.knotsV).not.toContain(4.5)
  if (op === 'open_exact_periodic_surface_axis') expect(definition.periodicV).toBe(false)
  if (op === 'remove_exact_periodic_surface_knot') expect(definition.knotsU).not.toContain(4.5)
  if (op === 'remove_exact_periodic_surface_seam_knot') expect(definition.knotsV).not.toContain(0.5)
})

it.each([
  ['rotate_periodic_surface_at_exact_knot(direction:"u",parameter:0.5,max_work:1000000)', /refused|knot/i],
  ['repeat_exact_periodic_surface_period(direction:"u",turns:2,max_work:1)', /refused|budget/i],
  ['partition_exact_periodic_surface_axis(direction:"u",seam:0,cuts:[1],part:2,max_work:1000000)', /index outside/],
  [
    'partition_exact_doubly_periodic_surface(seams:[0,0],u_cuts:[1],v_cuts:[2],u_index:0,v_index:2,max_work:1000000)',
    /index outside/,
  ],
])('propagates the native refusal from %s', async (edit, reason) => {
  const { document } = compileRushFrontend(chart + 'show chart.' + edit + '.tessellate(8,8)')
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(reason as RegExp)
})
