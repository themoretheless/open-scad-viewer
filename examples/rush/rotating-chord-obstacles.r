// @rush/1
a = bezier_curve(points:[[0,0,0],[0,0.1mm,0]])
b = bezier_curve(points:[[1,0,0],[1,0.1,0]])
result = a.rotating_chord_obstacles(inputs:[b],radius_profile:[[0,0],[1,0]],obstacles:[[[1,0,0.1],[-1,0,0.1]]],vertex_offsets:[[-2,0],[2,0]],parameter_tolerance:0.01,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
