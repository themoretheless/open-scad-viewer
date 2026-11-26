// @rush/1
roller = brep_cylinder(radius:2mm,height:3mm)
pin = brep_box([0,0,0],[1,1,1])
base = roller.make_step_assembly(inputs:[pin],definition_indices:[0,1,0,0],placements:[[[-2,0,0,0],[0,3,0,0],[0,0,4,0],[0,0,0,1]],[[1,0,0,20],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[1,0,0,40],[0,1,0,0],[0,0,1,0],[0,0,0,1]],[[-2,0,0,60],[0,3,0,0],[0,0,4,0],[0,0,0,1]]],definition_tolerances_mm:[],preview_index:0)
named = base.name_step_parts(product_name:"Drive",part_names:["Roller","Pin"],preview_index:0)
baked = named.bake_step_scales(preview_index:3)
show baked.brep_tessellate(8)
