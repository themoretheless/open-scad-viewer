// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
result = chart.tensor_axis_bounds(tensor_bounds:[[[2, 4], [0.5, 1], [0, 0]], [[0.5, 1], [6, 8], [0, 0]], [[0, 0], [0, 0], [9, 9]]],axis_vector:[1,1,0])
show result.tessellate(8,4)
