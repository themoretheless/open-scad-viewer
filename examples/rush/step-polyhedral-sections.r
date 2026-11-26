// @rush/1
box = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[],definition_indices:[0,0],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
result = assembly.step_polyhedral_sections(plane_normal:[0,0,1],plane_offset_mm:1,tolerance_uv:0.0000001,max_work:1000000,preview_index:0)
show result.brep_tessellate(3)
