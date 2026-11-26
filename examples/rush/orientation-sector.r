// @rush/1
a = bezier_curve(points:[[1,0,0],[1,1,0]])
result = a.orientation_sector(domain:[0,1],rays:[[1,-1],[1,0.5]],parameter_tolerance:0.005,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
