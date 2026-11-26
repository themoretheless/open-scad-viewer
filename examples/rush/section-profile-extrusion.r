// @rush/1
result = section_profile_extrusion(query_path_mm:[[0,0,0],[3,0,0],[3,3,0],[0,3,0],[0,0,0]],holes_mm:[],other_path_mm:[[1,1,0],[4,1,0],[4,4,0],[1,4,0],[1,1,0]],other_holes_mm:[],boolean_mode:"intersection",sweep_mm:[1,0,3],cut_normal:[0,1,1],cut_offset:3,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
