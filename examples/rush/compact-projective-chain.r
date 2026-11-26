// @rush/1
a = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 1],control_points:[[0mm,0mm,0mm],[0.5mm,0mm,0mm],[1mm,1mm,0mm]],weights:[1, 1, 1])
b = nurbs_curve(degree:2,knots:[10, 10, 10, 12, 12, 12],control_points:[[1mm,1mm,0mm],[1.5mm,2mm,0mm],[2mm,4mm,0mm]],weights:[3, 3, 3])
c = nurbs_curve(degree:2,knots:[30, 30, 30, 34, 34, 34],control_points:[[2mm,4mm,0mm],[2.5mm,6mm,0mm],[3mm,9mm,0mm]],weights:[3, 3, 3])
result = a.compact_projective_chain(inputs:[b,c],max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(32,4)
