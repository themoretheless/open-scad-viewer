// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.minimum_capsule_gap(domain:[0,1],capsules:[[0.5,-0.1,0,0.5,0.1,0,0.1]],radius_mm:0.05,gap_tolerance_mm:0.001,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
