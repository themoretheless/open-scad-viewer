// @rush/1
a = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0mm,0,0],[1mm,0,0]],weights:[1,2])
b = nurbs_curve(degree:1,knots:[1,1,2,2],control_points:[[1mm,0,0],[2mm,0,0]],weights:[3,5])
joined = a.merge_weighted_curves(target_geometry:b,max_work:1000000)
show joined.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
