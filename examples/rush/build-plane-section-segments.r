// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,2,0]],[[2,0,0],[2,2,0]]])
border = nurbs_curve(degree:1,knots:[3,3,4,5,6,7,7],control_points:[[0,0],[1,0],[1,1],[0,1],[0,0]],weights:[1,1,1,1,1])
hole = nurbs_curve(degree:1,knots:[3,3,4,5,6,7,7],control_points:[[0.25,0.25],[0.25,0.75],[0.75,0.75],[0.75,0.25],[0.25,0.25]],weights:[1,1,1,1,1])
other = bezier_surface(points:[[[1,0,-1],[1,0,1]],[[1,2,-1],[1,2,1]]])
audit = panel.build_plane_section_segments(target_geometry:other,loops:[[border],[hole],[border]],left_loop_count:2,parameter_width:0.00001,tolerance_uv:0.00001,max_winding_cells:4096,max_work:1000000)
show audit.tessellate(8,8)
