// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]])
uv = nurbs_curve(degree:1,knots:[2,2,6,6],control_points:[[1,0],[1,1]],weights:[1,1])
other = bezier_surface(points:[[[1,0,0],[1,1,0]],[[2,0,0],[2,1,0]]])
border = nurbs_curve(degree:1,knots:[2,2,6,6],control_points:[[0,0],[0,1]],weights:[1,1])
audit = panel.compare_uv_tangents(target_geometry:other,inputs:[uv,border],max_work:1000000)
show audit.tessellate(8,4)
