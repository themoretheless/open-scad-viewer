// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
b = bezier_curve(points:[[1,0,0],[0,1,0]])
result = a.minimum_profiled_rotating_gap(inputs:[b],radius_profile:[[0,0.02],[0.5,0.05],[1,0.1]],planes:[[0,1,0.15]],vertex_offsets:[[0,0],[0.25,0]],gap_tolerance_mm:0.001,max_cells:10000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
