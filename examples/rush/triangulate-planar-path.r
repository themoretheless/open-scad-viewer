// @rush/1
trace = triangulate_planar_path(query_path_mm:[[0,0,0],[6,0,0],[6,4,0],[0,4,0],[0,0,0]],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
