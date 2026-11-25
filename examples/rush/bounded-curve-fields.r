// @rush/1
// With u in [0,1] the source is (u,u*u,0).
source = bezier_curve(points:[[0,0,0],[0.5mm,0,0],[1mm,1mm,0]])
velocity = source.bounded_curve_derivative_field(order:1,piece:0,parameter:0.5,return_field:false,field_error:0.000001,max_work:1000000)
acceleration = velocity.bounded_curve_derivative_field(order:2,piece:0,parameter:0.5,return_field:false,field_error:0.000001,max_work:1000000)
speed = acceleration.bounded_curve_speed_squared_field(piece:0,parameter:0.5,return_field:false,field_error:0.000001,max_work:1000000)
normal_axis = speed.bounded_curve_binormal_field(piece:0,parameter:0.5,return_field:false,field_error:0.000001,max_work:1000000)
show normal_axis.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
