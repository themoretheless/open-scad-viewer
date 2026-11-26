// @rush/1
trace = measure_region_second_moments(query_path_mm:[[0,0,0],[6,0,0],[6,4,0],[0,4,0],[0,0,0]],holes_mm:[[[2,1,0],[4,1,0],[4,3,0],[2,3,0],[2,1,0]]],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
