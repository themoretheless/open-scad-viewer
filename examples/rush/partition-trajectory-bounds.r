// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.partition_trajectory_bounds(domain:[0,1],axis_vectors:[[1,0,0],[0,1,0]],allowed_ranges:[[0.25,0.75],[-1,1]],motion_limits:[[0.5,1,1],[1,1,1]],parameter_tolerance:0.01,max_cells:256)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
