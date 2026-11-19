// @rush/1
line = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0,0,0],[8mm,0,0]],weights:[1,1])
travel = line.curve_measure_directional_travel(axis_vector:[-1,0,0],interval:[0.25,0.75],tolerance_mm:0.001mm,max_cells:128,reverse_limit:3mm)
segments = travel.curve_measure_directional_travel_batch(axis_vector:[1,0,0],intervals:[[0,0.25],[0.25,1]],tolerance_mm:0.001mm,max_cells:128,reverse_limit:1mm)
show segments.surface_extrude(vector:[0,2mm,0]).tessellate(8,2)
