// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.partition_axis_position(domain:[0,1],axis_vector:[1,0,0],allowed:[0.25,0.75],parameter_tolerance:0.01,max_cells:256)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
