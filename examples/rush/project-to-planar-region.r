// @rush/1
trace = project_to_planar_region(query_path_mm:[[0,0,0],[2,0,2],[2,2,2],[0,2,0],[0,0,0]],query_point_mm:[0,1,2],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
