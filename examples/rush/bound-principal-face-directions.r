// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,2,0]],[[4,0,0],[4,2,0]]])
q0 = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[1,0.5],[1,1],[0.5,1]],weights:[1,1,2])
q1 = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0.5,1],[0,1],[0,0.5]],weights:[1,1,2])
q2 = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0,0.5],[0,0],[0.5,0]],weights:[1,1,2])
q3 = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0.5,0],[1,0],[1,0.5]],weights:[1,1,2])
audit = panel.bound_principal_face_directions(loops:[[q0,q1,q2,q3]],chart_origin:[2,1],moment_tolerances:[0.02,0.02,0.02],max_cells:8192,max_work:1000000)
show audit.tessellate(8,8)
