// @rush/1
trace = simplify_path_with_tolerance(query_path_mm:[[0,0,0],[1,"1/8",0],[2,0,0]],deviation_mm:"1/8",max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
