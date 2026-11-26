// @rush/1
trace = clip_path_to_planar_region(query_path_mm:[[0,0,0],[2,0,2],[2,2,2],[0,2,0],[0,0,0]],other_path_mm:[[-1,1,-1],[3,1,3]],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
