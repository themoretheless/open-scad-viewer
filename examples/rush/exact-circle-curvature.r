// @rush/1
circle = nurbs_curve(degree:2,knots:[-2,-2,-2,2,2,2],control_points:[[2mm,0,0],[2mm,2mm,0],[0,2mm,0]],weights:[1,1,2])
audit = circle.curve_curvature_squared_field(piece:0,parameter:0,return_field:false,max_work:1000000)
show audit.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
