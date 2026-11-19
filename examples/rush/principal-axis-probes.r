// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
axes = chart.surface_measure_trimmed_mass(contours:[[[0.125,0.125],[0.1875,0.125],[0.1875,0.1875],[0.125,0.1875],[0.125,0.125]],[[0.125,0.8125],[0.1875,0.8125],[0.1875,0.875],[0.125,0.875],[0.125,0.8125]]],component_densities:[[1,6,6],[0,2,2]],tolerance_mm2:0.00001,tolerance_uv:0.0000000001,max_work:1000000,max_surface_cells:20000,max_trim_cells:1000000,moment_tolerance:0.00000001,principal_axes:true,probe_point:[1,0,1],probe_direction:[1,0,1])
show axes.tessellate(8,4)
