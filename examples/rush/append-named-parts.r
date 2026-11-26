// @rush/1
pin = brep_box([0,0,0],[1,1,2])
mount = brep_box([0,0,0],[3,2,1])
assembly = pin.make_step_assembly(inputs:[mount],definition_indices:[0,1,0],placements:[[[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]], [[1, 0, 0, 5], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]], [[1, 0, 0, 10], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]],definition_tolerances_mm:[],preview_index:0)
named = assembly.name_step_parts(product_name:"Fixture",part_names:["Pin","Mount"],preview_index:0)
result = named.append_named_parts(target_geometry:named,placement:[[1, 0, 0, 20], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]],preview_index:0)
show result.brep_tessellate(3)
