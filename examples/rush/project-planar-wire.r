// @rush/1
panel = bezier_surface(points:[[[0,0,2],[0,2,2]],[[2,0,3],[2,2,3]]])
other = bezier_surface(points:[[[4,0,4],[4,2,4]],[[6,0,5],[6,2,5]]])
border = nurbs_curve(degree:1,knots:[3,3,4,5,6,7,7],control_points:[[0,0],[1,0],[1,1],[0,1],[0,0]],weights:[1,1,1,1,1])
wire = panel.project_planar_wire(loops:[[border],[border]],inputs:[other],contour_counts:[1,1],patch_index:1,loop_index:0,piece:0,max_work:1000000)
show wire.embed_xy_curve(height:0mm).surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
