// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
b = bezier_curve(points:[[1mm,0,0],[2mm,0,0]])
result = a.first_chain_violation(inputs:[b],domains:[[0,1],[0,1]],durations:[2,4],obstacles:[[1,2,0,2,3,1],[1.5,0,0,0.1]],margin_mm:0.1,radius_mm:0.05,parameter_tolerance:0.001,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
