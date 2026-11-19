// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
part1 = chart.lift_exact_periodic_surface_region_components(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],max_work:1000000)
part2 = part1.build_exact_periodic_surface_trim_components(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],max_work:1000000)
part3 = part2.build_exact_periodic_surface_trim_patches(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],max_work:1000000)
part4 = part3.build_exact_periodic_surface_trim_edges(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],max_work:1000000)
show part4.tessellate(8,4)
