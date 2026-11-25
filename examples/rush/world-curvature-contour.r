// @rush/1
panel = nurbs_surface(degree_u:2,degree_v:1,knots_u:[2,2,2,6,6,6],knots_v:[7,7,9,9],control_points:[[[0.25mm,0,0.0625mm],[0.25mm,1mm,0.0625mm]],[[0.625mm,0,0.25mm],[0.625mm,1mm,0.25mm]],[[1mm,0,1mm],[1mm,1mm,1mm]]],weights:[[1,1],[1,1],[1,1]])
result0 = panel.surface_gap_world_length(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,transverse:[7,8,9],max_depth:12,max_work:1000000)
result1 = result0.surface_gap_world_distance(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,transverse:[7,8,9],max_depth:12,probe_point:[0,0.5mm,0],max_work:1000000)
result2 = result1.surface_gap_world_bounds(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,transverse:[7,8,9],max_depth:12,max_work:1000000)
result3 = result2.surface_gap_world_polyline(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,transverse:[7,8,9],max_depth:12,return_field:true,max_work:1000000)
show result3.surface_extrude(vector:[0,0,0.1mm]).tessellate(8,4)
