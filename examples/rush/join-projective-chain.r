// @rush/1
a = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 1],control_points:[[0mm,0mm,0mm],[0.5mm,0mm,0mm],[1mm,1mm,0mm]],weights:[1, 1, 1])
b = nurbs_curve(degree:2,knots:[10, 10, 10, 12, 12, 12],control_points:[[1mm,1mm,0mm],[1.5mm,2mm,0mm],[2.5mm,5mm,0mm]],weights:[3, 3, 3])
c = nurbs_curve(degree:2,knots:[40, 40, 40, 41, 41, 41],control_points:[[2.5mm,5mm,0mm],[3mm,6.5mm,0mm],[3.625mm,8.5mm,0mm]],weights:[3, 3, 3])
result = a.join_projective_chain(inputs:[b,c],max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(32,4)
