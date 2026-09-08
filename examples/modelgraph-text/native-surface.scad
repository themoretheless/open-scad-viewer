// @modelgraph-text/1
// Исходная NURBS-поверхность сохраняется; сетка отображения строится отдельно.
param height = 8mm range 0mm..20mm
patch = nurbs_surface(
  degree_u: 2, degree_v: 2,
  knots_u: [0,0,0,1,1,1], knots_v: [0,0,0,1,1,1],
  control_points: [
    [[0mm,0mm,0mm],[0mm,10mm,0mm],[0mm,20mm,0mm]],
    [[10mm,0mm,0mm],[10mm,10mm,height],[10mm,20mm,0mm]],
    [[20mm,0mm,0mm],[20mm,10mm,0mm],[20mm,20mm,0mm]]
  ],
  weights: [[1,1,1],[1,1,1],[1,1,1]]
)
show patch
