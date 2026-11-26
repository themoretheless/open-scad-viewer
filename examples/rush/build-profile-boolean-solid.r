// @rush/1
result = build_profile_boolean_solid(query_path_mm:[[0,0,0],[3,0,0],[3,3,0],[0,3,0],[0,0,0]],holes_mm:[],other_path_mm:[[1,1,0],[4,1,0],[4,4,0],[1,4,0],[1,1,0]],other_holes_mm:[],boolean_mode:"intersection",sweep_mm:[1,0,3],tolerance_mm:0.00000001,tolerance_uv:0.00000001,max_work:1000000)
show result.brep_tessellate(4)
