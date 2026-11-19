// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
measure1 = chart.trimmed_surface_area_in_window(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],outer_contour:0,max_work:1000000,tolerance_uv:0.0000000001,tolerance_mm2:0.005,max_surface_cells:10000,max_trim_cells:1000000,window:[[3.75,4.25],[0.375,0.625]])
measure2 = measure1.trimmed_surface_centroid(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],outer_contour:0,max_work:1000000,tolerance_uv:0.0000000001,tolerance_mm2:0.005,max_surface_cells:10000,max_trim_cells:1000000)
measure3 = measure2.trimmed_surface_area_inertia(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],outer_contour:0,max_work:1000000,tolerance_uv:0.0000000001,tolerance_mm2:0.005,max_surface_cells:10000,max_trim_cells:1000000,origin:[0mm,0mm,0mm])
show measure3.tessellate(8,4)
