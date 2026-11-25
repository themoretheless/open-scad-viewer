// @rush/1
edge0 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0mm,0mm],[10mm,0mm]],weights:[1,1])
edge1 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[10mm,0mm],[10mm,10mm]],weights:[1,1])
edge2 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[10mm,10mm],[0mm,10mm]],weights:[1,1])
edge3 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0mm,10mm],[0mm,0mm]],weights:[1,1])
path = edge0.planar_toolpath_edge(tool_radius_mm:1mm,stock_mm:0.2mm,stepover_mm:0.5mm,passes:3,tolerance_mm:0.02mm,pass_index:2,piece:0,inputs:[edge1,edge2,edge3])
show path.embed_xy_curve(height:0mm).surface_extrude(vector:[0mm,0mm,1mm]).tessellate(16,4)
