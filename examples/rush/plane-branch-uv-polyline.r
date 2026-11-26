// @rush/1
panel = bezier_surface(points:[[[0,0,-1],[0,1,-1]],[[1,0,1],[1,1,1]]])
audit = panel.plane_branch_uv_polyline(plane_normal:[0,0,1],plane_offset_mm:0,knot_span:[1,1],direction:"u",tolerance_uv:0.00001,max_segments:16,max_work:1000000)
show audit.tessellate(8,8)
