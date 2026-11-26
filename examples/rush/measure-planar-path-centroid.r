// @rush/1
trace = measure_planar_path_centroid(query_path_mm:[[0,0,0],[2,0,2],[2,2,2],[0,2,0],[0,0,0]],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
