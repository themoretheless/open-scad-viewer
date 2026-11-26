// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
b = bezier_curve(points:[[1mm,0,0],[0,0,0]])
c = bezier_curve(points:[[0.5mm,-0.5mm,0],[0.5mm,0.5mm,0]])
result = a.tool_pair_intervals(inputs:[b,c],windows:[[0,1],[0,1],[0,1]],start_times:[0,0,0],durations:[1,1,1],radii_mm:[0.125,0.125,0.125],progress_tolerance:0.001,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
