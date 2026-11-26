// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.ball_workspace(domain:[0,1],planes:[[1,0,0,0.75]],radius_mm:0.05,parameter_tolerance:0.005,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
