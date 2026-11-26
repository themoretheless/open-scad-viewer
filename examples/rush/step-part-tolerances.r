// @rush/1
box = brep_box([0,0,0],[2,2,2])
other = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[other],definition_indices:[0,1],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[0.00001,0.0001],preview_index:0)
result = assembly.step_part_tolerances(preview_index:0)
show result.brep_tessellate(3)
