// @rush/1
box = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[],definition_indices:[0,0],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
result = assembly.step_clip_material(bounds_mm:[[0,0,0],[1,2,2]],max_candidates:8,piece:0)
show result.brep_tessellate(3)
