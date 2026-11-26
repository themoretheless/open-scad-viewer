// @rush/1
a = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0,0,0],[1,0,0]],weights:[1, 1])
b = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 1],control_points:[[1,0,0],[2,0,0],[2,2,0]],weights:[1, 0.5, 1])
c = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[2,2,0],[0,0,0]],weights:[1, 1])
replacement = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 1],control_points:[[1,0,0],[3,0,0],[2,2,0]],weights:[1, 0.5, 1])
result = a.edit_tangent_sector(inputs:[b,c,a,replacement,c],pieces_per_sector:3,sector_index:0,read_sector:0,piece:1)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
