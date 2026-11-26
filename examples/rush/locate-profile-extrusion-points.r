// @rush/1
result = locate_profile_extrusion_points(query_path_mm:[[0,0,0],[3,0,0],[3,3,0],[0,3,0],[0,0,0]],holes_mm:[],other_path_mm:[[1,1,0],[4,1,0],[4,4,0],[1,4,0],[1,1,0]],other_holes_mm:[],boolean_mode:"intersection",sweep_mm:[1,0,3],samples_mm:[["5/2",2,"3/2"],["3/2",2,"3/2"],[0,2,"3/2"]],max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
