// @rush/1
panel = nurbs_surface(degree_u:1,degree_v:1,knots_u:[2,2,4,4],knots_v:[5,5,7,10,10],control_points:[[[0,0,-1],[0,2,-1],[0,5,-1]],[[1,0,1],[1,2,1],[1,5,1]]],weights:[[1,1,1],[1,1,1]])
audit = panel.trace_plane_strip(plane_normal:[0,0,1],plane_offset_mm:0,direction:"u",solved_span:1,transverse_spans:[1,2],parameter_width:0.000001,tolerance_mm:0.00001,max_segments_per_cell:8,max_work:1000000)
show audit.tessellate(8,8)
