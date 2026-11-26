// @rush/1
a = bezier_curve(points:[[1,0,0],[1,1,0]])
result = a.angular_motion_limits(domain:[0,1],limits:[0.5,10,10],parameter_tolerance:0.005,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
