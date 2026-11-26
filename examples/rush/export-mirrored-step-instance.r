// @rush/1
box = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[],definition_indices:[0,0],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
result = assembly.set_scaled_step_placement(index:1,placement:[[-1,0,0,7],[0,1,0,0],[0,0,1,0],[0,0,0,1]],preview_index:1)
show result.brep_tessellate(3)
