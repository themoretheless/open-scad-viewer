// @rush/1
box = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[],definition_indices:[0,0],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
result = assembly.step_free_space(bounds_mm:[[-1,-1,-1],[4,3,3]],max_candidates:8)
show result.brep_tessellate(3)
