// @rush/1
trace = intersect_transverse_profiles(query_path_mm:[[0,0,0],[6,0,0],[6,4,0],[0,4,0],[0,0,0]],holes_mm:[[[2,1,0],[4,1,0],[4,3,0],[2,3,0],[2,1,0]]],other_path_mm:[[-1,2,-1],[7,2,-1],[7,2,1],[-1,2,1],[-1,2,-1]],other_holes_mm:[],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
