// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,2,0]],[[4,0,0],[4,2,0]]])
cut = panel.split_cell_by_plane(plane_normal:[1,0,0],plane_offset_mm:1,knot_span:[1,1],direction:"u",tolerance_mm:0.00001,max_segments:16,gap_tolerance_mm:0,positive_side:false,max_work:1000000)
show cut.tessellate(16,16)
