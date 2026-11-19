// @rush/1
profile = bezier_curve(points:[[0,0,0],[4mm,0,0],[8mm,0,0]])
split = profile.curve_exact_insert_knot(parameter:0.5,count:1,max_work:1000000)
compact = split.curve_exact_remove_knot(parameter:0.5,max_work:1000000).curve_exact_reduce_degree(max_work:1000000)
panel = compact.surface_extrude(vector:[0,0,4mm]).surface_edit(degree_v:2)
refined = panel.surface_exact_insert_knot(direction:"u",parameter:0.5,count:1,max_work:1000000)
restored = refined.surface_exact_remove_knot(direction:"u",parameter:0.5,max_work:1000000)
show restored.surface_exact_reduce_degree(direction:"v",max_work:1000000).tessellate(8,8)
