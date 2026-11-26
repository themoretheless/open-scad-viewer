// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.minimum_mixed_gap(domain:[0,1],obstacles:[[1,2,0,2,3,1],[0.5,0,0,0.1]],radius_mm:0.05,gap_tolerance_mm:0.001,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
