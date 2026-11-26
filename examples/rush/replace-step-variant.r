// @rush/1
box = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[],definition_indices:[0,0],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
part = brep_box([0,0,0],[1,1,1])
result = assembly.replace_step_variant(selection:[1],target_geometry:part,preview_index:0)
show result.brep_tessellate(3)
