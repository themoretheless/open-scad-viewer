// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]])
uv = nurbs_curve(degree:1,knots:[2,2,6,6],control_points:[[1,0],[1,1]],weights:[1,1])
edge = nurbs_curve(degree:1,knots:[2,2,6,6],control_points:[[1,0,0],[1,1,0]],weights:[1,1])
audit = panel.verify_uv_edge(target_geometry:edge,inputs:[uv],max_work:1000000)
show audit.tessellate(8,4)
