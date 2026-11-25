// @rush/1
panel = nurbs_surface(degree_u:2,degree_v:1,knots_u:[2,2,2,6,6,6],knots_v:[7,7,9,9],control_points:[[[0.25mm,0,0.0625mm],[0.25mm,1mm,0.0625mm]],[[0.625mm,0,0.25mm],[0.625mm,1mm,0.25mm]],[[1mm,0,1mm],[1mm,1mm,1mm]]],weights:[[1,1],[1,1],[1,1]])
audit0 = panel.surface_gap_uv_polyline(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,transverse:[7,8,9],max_depth:12,max_work:1000000)
audit1 = audit0.surface_gap_adaptive_polyline(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,uv_error:0.01,max_vertices:64,max_depth:12,max_work:1000000)
audit2 = audit1.surface_gap_second_derivative(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,max_work:1000000)
audit3 = audit2.surface_gap_second_order_polyline(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,uv_error:0.01,max_vertices:64,max_depth:12,max_work:1000000)
show audit3.tessellate(16,8)
