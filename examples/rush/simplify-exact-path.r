// @rush/1
trace = simplify_exact_path(query_path_mm:[[0,0,0],["1/2",0,0],[1,0,0],[1,0,0],[0,0,0]],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
