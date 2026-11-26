// @rush/1
panel = nurbs_surface(degree_u:1,degree_v:1,knots_u:[0,0,1,1],knots_v:[-1,0,1,2,3,4,5],control_points:[[[1,0,-1],[0,1,-1],[-1,0,-1],[0,-1,-1],[1,0,-1]],[[1,0,1],[0,1,1],[-1,0,1],[0,-1,1],[1,0,1]]],weights:[[1,1,1,1,1],[1,1,1,1,1]],periodic_v:true)
audit = panel.store_closed_plane_piece(plane_normal:[0,0,1],plane_offset_mm:0,direction:"u",solved_span:1,transverse_spans:[1,4],parameter_width:0.000001,tolerance_mm:0.00001,max_segments_per_cell:8,gap_tolerance_mm:0,piece:3,use_pcurve:false,max_work:1000000)
show audit.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
