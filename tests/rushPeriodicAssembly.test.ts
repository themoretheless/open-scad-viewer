import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const stripProgram = readFileSync('examples/rush/exact-periodic-strip-assembly.r', 'utf8')
const chart = readFileSync('examples/rush/exact-periodic-window.r', 'utf8').split('patch =')[0]!
const weightScale = (rows: number, columns: number) =>
  '.surface_edit_weights(edits:[' +
  Array.from({ length: rows }, (_, i) => Array.from({ length: columns }, (_, j) => '[' + i + ',' + j + ',2]'))
    .flat()
    .join(',') +
  '])'
const union = 'assemble_periodic_surface_axis_with_exact_knot_union'
const firstKnot = '.surface_exact_insert_knot(direction:"v",parameter:0.25,count:1,max_work:1000000)'
const secondKnot = firstKnot.replace('0.25', '0.75')

it.each([
  [union, firstKnot, secondKnot],
  ['assemble_exact_periodic_surface_axis', '', ''],
  ['assemble_periodic_surface_axis_with_exact_weight_scaling', '', weightScale(3, 2)],
  ['assemble_periodic_surface_axis_with_exact_basis_matching', '', '.surface_edit(degree_v:2)'],
  ['assemble_periodic_surface_axis_with_exact_degree_matching', '', '.surface_edit(degree_u:2)'],
])('reassembles both source strips with %s', async (op, first, second) => {
  const source = stripProgram.replace(union, op!).replace(firstKnot, first!).replace(secondKnot, second!)
  await verifyAssembly(source, op!, false)
})

function gridProgram(op: string) {
  const parts = [
    [0, 0],
    [0, 1],
    [1, 0],
    [1, 1],
  ].map(
    ([u, v], i) =>
      'p' +
      i +
      ' = chart.partition_exact_doubly_periodic_surface(seams:[0,0],u_cuts:[2],v_cuts:[2],u_index:' +
      u +
      ',v_index:' +
      v +
      ',max_work:1000000)' +
      (op.includes('scaling') && i === 3 ? weightScale(3, 3) : ''),
  )
  return (
    chart +
    parts.join('\n') +
    '\nassembled = ' +
    op +
    '(p0,p1,p2,p3,columns:2,max_work:1000000)\nshow assembled.tessellate(8,8)'
  )
}
it.each(['assemble_exact_doubly_periodic_surface', 'assemble_doubly_periodic_surface_with_exact_weight_scaling'])(
  'reassembles four independently authored patches with %s',
  async op => verifyAssembly(gridProgram(op), op, true),
)

async function verifyAssembly(source: string, op: string, grid: boolean) {
  const { document } = compileRushFrontend(source)
  const original = document.nodes.find(n => n.op === (grid ? 'surface' : 'surface_extrude'))!.id
  const assembled = document.nodes.find(n => n.op === op)!.id
  const evaluations = [0.375, 3.625].flatMap(u => [original, assembled].map(node => ({ node, u, v: grid ? u : 0.625 })))
  const result = await buildOwnNurbsAsync(document, { action: 'evaluate', evaluations })
  expect(result.report.definitions[assembled]).toMatchObject({ periodicU: true, periodicV: grid })
  for (const i of [0, 2]) expect(result.evaluations[i + 1]!.point).toEqual(result.evaluations[i]!.point)
}

it.each([
  [stripProgram.replace('part:1', 'part:0'), /refused/i],
  [stripProgram.replace('a,b,direction:"u",max_work:1000000', 'a,b,direction:"u",max_work:1'), /refused|budget/i],
  [gridProgram('assemble_exact_doubly_periodic_surface').replace('columns:2', 'columns:3'), /rectangular/],
  [
    gridProgram('assemble_doubly_periodic_surface_with_exact_weight_scaling').replace(
      'assemble_doubly_periodic_surface_with_exact_weight_scaling(',
      'assemble_exact_doubly_periodic_surface(',
    ),
    /refused/i,
  ],
] as const)('rejects incompatible boundaries, invalid grids and exhausted work', async (source, reason) => {
  const { document } = compileRushFrontend(source)
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(reason)
})
