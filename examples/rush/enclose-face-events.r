// @rush/1
panel = bezier_surface(points:[[[0,-4,0],[0,4,0]],[[1,-4,0],[1,4,0]]])
other = bezier_surface(points:[[[0,0,-4],[0,0,4]],[[1,0,-4],[1,0,4]]])
arc = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0,0.5625],[0.5,0.1875],[1,0.9375]],weights:[1,1,1])
back = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[1,0.9375],[0,0.5625]],weights:[1,1])
reverse_arc = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[1,0.9375],[0.5,0.1875],[0,0.5625]],weights:[1,1,1])
close_arc = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0,0.5625],[1,0.9375]],weights:[1,1])
audit = panel.enclose_face_events(target_geometry:other,loops:[[arc,back],[reverse_arc,close_arc]],left_loop_count:1,xyz_tolerance:0.001,max_work:1000000)
show audit.tessellate(8,8)
