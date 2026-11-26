// @rush/1
trace = extrude_profile_boolean_shell(query_path_mm:[[0,0,0],[6,0,0],[6,4,0],[0,4,0],[0,0,0]],holes_mm:[[[2,1,0],[4,1,0],[4,3,0],[2,3,0],[2,1,0]]],other_path_mm:[[3,0,0],[7,0,0],[7,4,0],[3,4,0],[3,0,0]],other_holes_mm:[[["9/2",1,0],["11/2",1,0],["11/2",3,0],["9/2",3,0],["9/2",1,0]]],boolean_mode:"intersection",sweep_mm:[1,0,3],max_work:1000000)
show trace.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
