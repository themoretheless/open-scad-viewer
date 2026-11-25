// @rush/1
profile = nurbs_curve(degree:1,knots:[2,2,4,7,7],control_points:[[0mm,0mm],[10mm,0mm],[10mm,10mm]],weights:[0.7,1.3,0.9])
result = profile.planar_knot_miter(knot:4,distance_mm:0.25mm,extension_limit_mm:1mm,tolerance_mm:0.00001mm)
show result.embed_xy_curve(height:2mm).surface_extrude(vector:[0mm,0mm,1mm]).tessellate(16,4)
