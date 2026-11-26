// @rush/1
result = mesh_parallel_sweep_overlap(query_path_mm:[[0,0,0],[3,0,0],[3,3,0],[0,3,0],[0,0,0]],holes_mm:[],other_path_mm:[[1,1,4],[4,1,4],[4,4,4],[1,4,4],[1,1,4]],other_holes_mm:[],sweep_mm:[0,0,3],other_sweep_mm:[0,0,-3],max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
