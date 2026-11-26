// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]])
uv = nurbs_curve(degree:1,knots:[2,2,6,6],control_points:[[1,0],[1,1]],weights:[1,1])
other = bezier_surface(points:[[[1,0,0],[1,1,0]],[[2,0,0],[2,1,1]]])
border = nurbs_curve(degree:1,knots:[2,2,6,6],control_points:[[0,0],[0,1]],weights:[1,1])
audit = panel.isolate_seam_angle(target_geometry:other,inputs:[uv,border],min_cosine_squared:0.8,parameter_width:0.0001,max_work:1000000)
show audit.tessellate(8,4)
