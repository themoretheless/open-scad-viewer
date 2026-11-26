// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.hull_obstacles(domain:[0,1],obstacles:[[[1,0,0,0.6],[-1,0,0,-0.4]]],radius_mm:0.05,vertex_offsets:[[0,0,0],[0.05,0,0],[0,0.05,0]],parameter_tolerance:0.005,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
