// @rush/1
panel = bezier_surface(points:[[[0,0,-1],[0,1,-1]],[[1,0,1],[1,1,1]]])
audit = panel.sample_plane_branch(plane_normal:[0,0,1],plane_offset_mm:0,knot_span:[1,1],direction:"u",transverse_ratios:[[0,1],[1,3],[1,1]],parameter_width:0.00001,max_work:1000000)
show audit.tessellate(8,8)
