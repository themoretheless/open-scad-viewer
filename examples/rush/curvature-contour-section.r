// @rush/1
panel = nurbs_surface(degree_u:2,degree_v:1,knots_u:[2,2,2,6,6,6],knots_v:[7,7,9,9],control_points:[[[0.25mm,0,0.0625mm],[0.25mm,1mm,0.0625mm]],[[0.625mm,0,0.25mm],[0.625mm,1mm,0.25mm]],[[1mm,0,1mm],[1mm,1mm,1mm]]],weights:[[1,1],[1,1],[1,1]])
section = panel.surface_gap_plane_section(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,transverse:[7,8,9],max_depth:12,plane_normal:[0,1,0],plane_offset_mm:0.25mm,max_work:1000000)
show section.tessellate(8,4)
