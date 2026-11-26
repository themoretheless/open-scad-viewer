// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
b = bezier_curve(points:[[1mm,0,0],[0,0,0]])
result = a.affine_contact_intervals(inputs:[b],windows:[[0,1],[0.25,0.75]],radii_mm:[0.05,0.1],progress_tolerance:0.001,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
