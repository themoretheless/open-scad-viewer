// @rush/1
curve = nurbs_curve(degree:3,knots:[0,0,0,0,1,1,1,1],control_points:[[0,0,0],[0,0,0],[0,0,0],[8mm,0,0]],weights:[1,1,1,1])
peak1 = curve.curve_axial_derivative_peak(axis_vector:[1,0,0],interval:[0,1],order:1,tolerance_mm:0.01mm,max_cells:2048)
locations1 = peak1.curve_axial_derivative_peak(axis_vector:[1,0,0],interval:[0,1],order:1,tolerance_mm:0.01mm,max_cells:2048,parameter_tolerance:0.05)
peak2 = locations1.curve_axial_derivative_peak(axis_vector:[1,0,0],interval:[0,1],order:2,tolerance_mm:0.01mm,max_cells:2048)
locations2 = peak2.curve_axial_derivative_peak(axis_vector:[1,0,0],interval:[0,1],order:2,tolerance_mm:0.01mm,max_cells:2048,parameter_tolerance:0.05)
peak3 = locations2.curve_axial_derivative_peak(axis_vector:[1,0,0],interval:[0,1],order:3,tolerance_mm:0.01mm,max_cells:2048)
locations3 = peak3.curve_axial_derivative_peak(axis_vector:[1,0,0],interval:[0,1],order:3,tolerance_mm:0.01mm,max_cells:2048,parameter_tolerance:0.05)
show locations3.surface_extrude(vector:[0,2mm,0]).tessellate(8,2)
