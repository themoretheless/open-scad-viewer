// @rush/1
curve = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0,0,0],[0,0,0],[8mm,0,0]],weights:[1,1,1])
speed = curve.curve_axis_motion_partition_speed(interval:[0,1],axis_vector:[1,0,0],speed_limit:8,parameter_tolerance:0.02,max_cells:128)
motion = speed.curve_axis_motion_partition_motion(interval:[0,1],directions:[[1,0,0]],limits:[[8,20,10]],parameter_tolerance:0.02,max_cells:128)
axes = motion.curve_axis_motion_partition_axes(interval:[0,1],directions:[[1,0,0],[0,1,0]],limits:[[8,20,10],[1,1,1]],parameter_tolerance:0.02,max_cells:256)
show axes.surface_extrude(vector:[0,2mm,0]).tessellate(8,2)
