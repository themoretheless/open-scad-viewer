// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.profiled_hull_workspace(planes:[[1,0,0,0.75]],radius_profile:[[0,0.05],[0.5,0.2],[1,0.1]],vertex_offsets:[[0,0,0],[0.05,0,0],[0,0.05,0]],parameter_tolerance:0.005,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
