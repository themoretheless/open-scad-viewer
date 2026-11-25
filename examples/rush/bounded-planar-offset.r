// @rush/1
profile = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0mm,0mm],[5mm,2mm],[10mm,0mm]],weights:[1,1,1])
result = profile.planar_offset_piece(distance_mm:0.25mm,tolerance_mm:0.05mm,max_cells:10000,piece:0)
show result.embed_xy_curve(height:2mm).surface_extrude(vector:[0mm,0mm,1mm]).tessellate(16,4)
