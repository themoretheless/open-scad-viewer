// @rush/1
box = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[],definition_indices:[0,0],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
result = assembly.step_ray_multiplicity(ray_origin:[-1,0.75,1.25],ray_direction:[1,0,0],minimum_occurrences:2,max_parameter:8,tolerance_uv:0.0000001,max_cells:100000,max_domain_cells:200000,preview_index:0)
show result.brep_tessellate(3)
