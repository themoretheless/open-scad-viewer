const nodes = [
  {
    id: 'patch', op: 'surface', degree_u: 1, degree_v: 1,
    knots_u: [0, 0, 1, 1], knots_v: [0, 0, 1, 1],
    control_points: [[[0, 0, 0], [0, 2, 0]], [[2, 0, 0], [2, 2, 0]]], weights: [[1, 1], [1, 1]],
  },
  { id: 'skin', op: 'tessellate', input: 'patch', segments_u: 2, segments_v: 3 },
  { id: 'a', op: 'thicken', input: 'skin', vector: [0, 0, 2] },
  { id: 'shifted', op: 'transform', input: 'patch', matrix: [[1, 0, 0, 1], [0, 1, 0, 1], [0, 0, 1, 1], [0, 0, 0, 1]] },
  { id: 'skin_b', op: 'tessellate', input: 'shifted', segments_u: 3, segments_v: 2 },
  { id: 'b', op: 'thicken', input: 'skin_b', vector: [0, 0, 2] },
]

export const nurbsProcessFixtures = [
  ...(['union', 'intersection', 'difference'] as const).map((operation, index) => ({
    name: operation,
    volume: [15, 1, 7][index]!,
    document: {
      language: 'modelgraph/nurbs-1', units: 'mm',
      nodes: [...nodes, { id: 'result', op: 'mesh_boolean', inputs: ['a', 'b'], operation }], root: 'result',
    },
  })),
  {
    name: 'large-stl', volume: 8,
    document: {
      language: 'modelgraph/nurbs-1', units: 'mm',
      nodes: [nodes[0], { ...nodes[1], segments_u: 48, segments_v: 48 }, nodes[2]], root: 'a',
    },
  },
]
