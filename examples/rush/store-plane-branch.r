// @rush/1
panel = bezier_surface(points:[[[0,0,-1],[0,1,-1]],[[1,0,1],[1,1,1]]])
audit = panel.store_plane_branch(plane_normal:[0,0,1],plane_offset_mm:0,knot_span:[1,1],direction:"u",tolerance_mm:0.00001,max_segments:16,max_work:1000000)
show audit.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
