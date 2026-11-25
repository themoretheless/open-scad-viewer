// @rush/1
panel = nurbs_surface(degree_u:1,degree_v:1,knots_u:[2,2,6,6],knots_v:[7,7,9,9],control_points:[[[0,0,0],[0,4mm,0]],[[4mm,0,0],[4mm,4mm,4mm]]],weights:[[1,1],[3,3]])
piece = panel.split_tensor_surface(direction:"u",parameter:4,part:1,max_work:1000000)
show piece.tessellate(8,8)
