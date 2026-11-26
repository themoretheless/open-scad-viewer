// @rush/1
trace = bound_path_principal_moments(query_path_mm:[[0,0,0],[6,0,0],[6,4,0],[0,4,0],[0,0,0]],tolerance_mm2:0.000001,max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
