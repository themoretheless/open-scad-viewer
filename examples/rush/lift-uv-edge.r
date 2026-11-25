// @rush/1
panel = nurbs_surface(degree_u:2,degree_v:1,knots_u:[0,0,0,2,4,4,4],knots_v:[0,0,4,4],control_points:[[[0,0,0],[0,4,0]],[[1,0,0],[1,4,4]],[[3,0,0],[3,4,12]],[[4,0,0],[4,4,16]]],weights:[[1,1],[1,1],[1,1],[1,1]])
trace = nurbs_curve(degree:1,knots:[3,3,7,7],control_points:[[0,0],[4,4]],weights:[1,1])
edge = trace.lift_uv_edge(target_geometry:panel,piece:1,max_work:1000000)
show edge.surface_extrude(vector:[0,1mm,0]).tessellate(8,4)
