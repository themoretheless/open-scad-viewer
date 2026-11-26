// @rush/1
left = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0,0,0],[1mm,0,0]],weights:[1,1])
right = nurbs_curve(degree:1,knots:[1,1,2,2],control_points:[[1mm,0,0],[3mm,0,0]],weights:[1,1])
result = left.compare_geometric_seam(target_geometry:right,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
