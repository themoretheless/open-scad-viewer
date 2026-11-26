// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
result = chart.physical_moment_gaps(tensor_bounds:[[[3, 3], [0, 0], [0, 0]], [[0, 0], [4, 4], [0, 0]], [[0, 0], [0, 0], [5, 5]]],moment_tolerance:0.000000001)
show result.tessellate(8,4)
