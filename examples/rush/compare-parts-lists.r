// @rush/1
pin = brep_box([0,0,0],[1,1,2])
mount = brep_box([0,0,0],[3,2,1])
assembly = pin.make_step_assembly(inputs:[mount],definition_indices:[0,1,0],placements:[[[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]], [[1, 0, 0, 5], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]], [[1, 0, 0, 10], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]],definition_tolerances_mm:[],preview_index:0)
named = assembly.name_step_parts(product_name:"Fixture",part_names:["Pin","Mount"],preview_index:0)
changed = named.select_step_parts(definition_indices:[1],product_name:"Mount only",preview_index:0)
result = named.compare_parts_lists(target_geometry:changed,preview_index:0)
show result.brep_tessellate(3)
