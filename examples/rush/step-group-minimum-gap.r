// @rush/1
box = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[],definition_indices:[0,0],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,5],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
result = assembly.step_group_minimum_gap(left_group:[0],right_group:[1],tolerance_mm:0.00001,tolerance_uv:0.0000001,max_cells:100000,max_domain_cells:200000,max_pairs:8,preview_index:0)
show result.brep_tessellate(3)
