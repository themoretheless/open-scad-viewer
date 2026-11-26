// @rush/1
a = bezier_curve(points:[[0,0,0],[0,0.001mm,0]])
b = bezier_curve(points:[[1,0,0],[1,0.001,0]])
result = a.rotating_skew_obstacles(inputs:[b],radius_profile:[[0,0.42],[1,0.42]],obstacles:[[[-1,0,-1.2],[0.8,-0.6,0.75]]],vertex_offsets:[[1,0]],parameter_tolerance:0.01,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
