// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
result = chart.trim_set_axis_bounds(axis_vector:[1,1,0],contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]],[[0.1875,0.375],[0.3125,0.375],[0.3125,0.625],[0.1875,0.625],[0.1875,0.375]]],tolerance_mm2:0.005,tolerance_uv:0.0000000001,max_work:1000000,max_surface_cells:20000,max_trim_cells:1000000)
show result.tessellate(8,4)
