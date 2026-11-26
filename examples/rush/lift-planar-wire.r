// @rush/1
panel = bezier_surface(points:[[[0,0,2],[0,2,2]],[[2,0,3],[2,2,3]]])
border = nurbs_curve(degree:1,knots:[3,3,4,5,6,7,7],control_points:[[4,0],[6,0],[6,2],[4,2],[4,0]],weights:[1,1,1,1,1])
wire = panel.lift_planar_wire(loops:[[border]],loop_index:0,piece:0,max_work:1000000)
show wire.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
