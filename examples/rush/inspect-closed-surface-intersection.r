// @rush/1
bowl = nurbs_surface(degree_u:2,degree_v:2,knots_u:[0,0,0,1,1,1],knots_v:[0,0,0,1,1,1],control_points:[[[0.0,0.0,0.4375],[0.0,0.5,-0.0625],[0.0,1.0,0.4375]],[[0.5,0.0,-0.0625],[0.5,0.5,-0.5625],[0.5,1.0,-0.0625]],[[1.0,0.0,0.4375],[1.0,0.5,-0.0625],[1.0,1.0,0.4375]]],weights:[[1,1,1],[1,1,1],[1,1,1]])
plane = nurbs_surface(degree_u:2,degree_v:2,knots_u:[0,0,0,1,1,1],knots_v:[0,0,0,1,1,1],control_points:[[[0.0,0.0,0.0],[0.0,0.5,0.0],[0.0,1.0,0.0]],[[0.5,0.0,0.0],[0.5,0.5,0.0],[0.5,1.0,0.0]],[[1.0,0.0,0.0],[1.0,0.5,0.0],[1.0,1.0,0.0]]],weights:[[1,1,1],[1,1,1],[1,1,1]])
checked = bowl.inspect_surface_intersection(target_geometry:plane)
show checked.tessellate(8,8)
