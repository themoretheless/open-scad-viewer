// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.variable_scene_clearance(domain:[0,1],obstacles:[[1,2,0,2,3,1],[0.5,0,0,0.1]],tool_radii:[0.05,0.15],parameter_tolerance:0.001,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
