// @rush/1
panel = nurbs_surface(degree_u:2,degree_v:1,knots_u:[2,2,2,6,6,6],knots_v:[7,7,9,9],control_points:[[[0.25mm,0,0.0625mm],[0.25mm,1mm,0.0625mm]],[[0.625mm,0,0.25mm],[0.625mm,1mm,0.25mm]],[[1mm,0,1mm],[1mm,1mm,1mm]]],weights:[[1,1],[1,1],[1,1]])
target = nurbs_surface(degree_u:2,degree_v:1,knots_u:[2,2,2,6,6,6],knots_v:[7,7,9,9],control_points:[[[0.25mm,0,1.0625mm],[0.25mm,1mm,1.0625mm]],[[0.625mm,0,1.25mm],[0.625mm,1mm,1.25mm]],[[1mm,0,2mm],[1mm,1mm,2mm]]],weights:[[1,1],[1,1],[1,1]])
distance = panel.surface_gap_pair_distance(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,transverse:[7,8,9],max_depth:12,target_geometry:target,target_knot_span:[2,1],target_direction:"u",target_gap_level_per_mm2:1/1mm/1mm,target_transverse:[7,7.5,9],target_max_depth:12,max_work:1000000)
show distance.tessellate(8,4)
