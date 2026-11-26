// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
b = bezier_curve(points:[[1,0,0],[0,1,0]])
result = a.rounded_rotating_obstacles(inputs:[b],domain:[0,1],obstacles:[[[-1,0,-0.8],[1,0,1.4],[0,-1,1],[0,1,1]]],vertex_offsets:[[0,0],[0.25,0]],radius_mm:0.05,parameter_tolerance:0.005,max_cells:10000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
