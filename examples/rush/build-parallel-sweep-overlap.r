// @rush/1
result = build_parallel_sweep_overlap(query_path_mm:[[0,0,0],[3,0,0],[3,3,0],[0,3,0],[0,0,0]],holes_mm:[],other_path_mm:[[1,1,4],[4,1,4],[4,4,4],[1,4,4],[1,1,4]],other_holes_mm:[],sweep_mm:[0,0,3],other_sweep_mm:[0,0,-3],tolerance_mm:0.00000001,tolerance_uv:0.00000001,max_work:1000000)
show result.brep_tessellate(4)
