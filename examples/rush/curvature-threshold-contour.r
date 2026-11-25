// @rush/1
panel = nurbs_surface(degree_u:2,degree_v:1,knots_u:[2,2,2,6,6,6],knots_v:[7,7,9,9],control_points:[[[0.25mm,0,0.0625mm],[0.25mm,1mm,0.0625mm]],[[0.625mm,0,0.25mm],[0.625mm,1mm,0.25mm]],[[1mm,0,1mm],[1mm,1mm,1mm]]],weights:[[1,1],[1,1],[1,1]])
roots = panel.surface_gap_edge_roots(knot_span:[2,1],source_edge:"v_min",gap_level_per_mm2:1/1mm/1mm,max_depth:10,max_work:1000000)
graph = roots.surface_gap_contour_graph(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,max_work:1000000)
samples = graph.surface_gap_contour_samples(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,transverse:[7,8,9],max_depth:10,max_work:1000000)
slope = samples.surface_gap_contour_slope(knot_span:[2,1],direction:"u",gap_level_per_mm2:1/1mm/1mm,max_work:1000000)
show slope.tessellate(16,8)
