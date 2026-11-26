// @rush/1
trace = locate_planar_path_points(query_path_mm:[[0,0,0],[2,0,2],[2,2,2],[0,2,0],[0,0,0]],other_path_mm:[[1,1,1],[2,1,2],[3,1,3],[0,1,2]],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
