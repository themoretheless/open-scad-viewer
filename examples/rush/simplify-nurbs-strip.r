// @rush/1
line = bezier_curve(points:[[0,0,0],[4mm,0,0],[8mm,0,0]])
split_line = line.curve_edit(insert_knots:[{value:0.5,count:1}])
profile = split_line.curve_remove_knot(parameter:0.5,max_deviation:0.01mm).curve_reduce_degree(degree:1,max_deviation:0.01mm).curve_rebuild(degree:1,control_count:2,max_deviation:0.01mm)
strip = profile.surface_extrude(vector:[0,0,4mm])
refined = strip.surface_edit(degree_v:2,insert_knots_u:[{value:0.5,count:1}])
panel = refined.surface_remove_knot(direction:"u",parameter:0.5,max_deviation:0.01mm).surface_reduce_degree(direction:"v",degree:1,max_deviation:0.01mm).surface_rebuild(direction:"v",degree:1,control_count:2,max_deviation:0.01mm)
show panel.tessellate(8,8)
