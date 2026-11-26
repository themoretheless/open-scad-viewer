// @rush/1
trace = intersect_spatial_paths(query_path_mm:[[0,0,0],[4,0,0]],other_path_mm:[[3,0,0],[1,0,0]],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
