// @rush/1
// With u in [0,1] the source is (u,u*u,0).
source = bezier_curve(points:[[0,0,0],[0.5mm,0,0],[1mm,1mm,0]])
velocity = source.curve_derivative_field(order:1,piece:0,parameter:0.5,return_field:false,max_work:1000000)
acceleration = velocity.curve_derivative_field(order:2,piece:0,parameter:0.5,return_field:false,max_work:1000000)
speed = acceleration.curve_speed_squared_field(piece:0,parameter:0.5,return_field:false,max_work:1000000)
normal_axis = speed.curve_binormal_field(piece:0,parameter:0.5,return_field:false,max_work:1000000)
show normal_axis.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
