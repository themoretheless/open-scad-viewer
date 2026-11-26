// @rush/1
part0 = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[2, 0, 0], [2, 1, 0]],weights:[1, 1])
part1 = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, 2, 0], [-1, 2, 0]],weights:[1, 1])
part2 = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[-2, 0, 0], [-2, -1, 0]],weights:[1, 1])
part3 = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, -2, 0], [1, -2, 0]],weights:[1, 1])
result = part0.compress_rotated_segments(inputs:[part1,part2,part3],sector_index:2,tolerance_mm:0.000001)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
