// @rush/1
a = nurbs_curve(degree:3,knots:[0,0,0,0,1,2,2,2,2],control_points:[[0,0,0],[0,0,0],[0,0,0],[0,0,0],[1mm,0,0]],weights:[1,1,1,1,1])
result = a.inspect_knot_jumps(axis_vector:[1,0,0],domain:[0.5,1.5],max_knots:1,jump_limits:[0,0,1])
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
