// @rush/1
panel = bezier_surface(points:[[[0,0,1],[0,1,1]],[[0.5,0,-0.125],[0.5,1,-0.125]],[[1,0,1],[1,1,1]]])
audit = panel.audit_plane_side(plane_normal:[0,0,1],plane_offset_mm:0,max_work:1000000)
show audit.tessellate(8,8)
