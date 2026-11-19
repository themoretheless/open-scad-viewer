// @rush/1
path = nurbs_curve(degree:1,knots:[2,2,3,4,4],control_points:[[0,0,0],[5mm,0,0],[2mm,0,0]],weights:[1,1,1])
total = path.curve_directional_travel_full(axis_vector:[1,0,0],tolerance_mm:0.001mm,max_cells:128)
signed = total.curve_directional_travel_signed(axis_vector:[1,0,0],tolerance_mm:0.001mm,max_cells:128)
checked = signed.curve_check_backtracking(axis_vector:[1,0,0],interval:[3,4],reverse_limit:2mm,tolerance_mm:0.001mm,max_cells:128)
show checked.surface_extrude(vector:[0,2mm,0]).tessellate(8,2)
