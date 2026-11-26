// @rush/1
box = brep_box([0,0,0],[2,2,2])
assembly = box.make_step_assembly(inputs:[],definition_indices:[0,0,0],placements:[[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,2],[0,1,0,0],[0,0,1,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
named = assembly.name_step_parts(product_name:"Fixture",part_names:["Pin"],preview_index:0)
styled = named.style_step_occurrences(group_names:["Left","Right"],parent_indices:[0,1,-1],selection:[0,1],style_names:["Blue","Red"],style_colours:[[0,0,1],[1,0,0]],preview_index:0)
show styled.brep_tessellate(3)
