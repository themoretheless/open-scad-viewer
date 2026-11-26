// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.variable_sphere_clearance(domain:[0,1],spheres:[[0.5,0,0,0.1]],tool_radii:[0.05,0.15],parameter_tolerance:0.005,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
