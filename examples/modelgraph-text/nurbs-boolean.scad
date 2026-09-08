// @modelgraph-text/1
param width = 20mm range 10mm..40mm
param thickness = 4mm range 1mm..8mm
patch = nurbs_surface(
  degree_u: 1, degree_v: 1,
  knots_u: [0,0,1,1], knots_v: [0,0,1,1],
  control_points: [[[0,0,0],[0,width,0]],[[width,0,0],[width,width,0]]],
  weights: [[1,1],[1,1]]
)
body = patch |> tessellate(2,2) |> thicken([0,0,thickness])
cutter = patch |> transform([[1,0,0,width/2],[0,1,0,width/2],[0,0,1,0],[0,0,0,1]])
  |> tessellate(2,2) |> thicken([0,0,thickness])
show body |> mesh_subtract(cutter)
