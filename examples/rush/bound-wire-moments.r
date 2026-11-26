// @rush/1
q0 = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[5,-2],[5,-1],[3,-1]],weights:[1,1,2])
q1 = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[3,-1],[1,-1],[1,-2]],weights:[1,1,2])
q2 = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[1,-2],[1,-3],[3,-3]],weights:[1,1,2])
q3 = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[3,-3],[5,-3],[5,-2]],weights:[1,1,2])
audit = q0.bound_wire_moments(inputs:[q1,q2,q3],loops:[],chart_origin:[3,-2],moment_tolerances:[0.02,0.02,0.02],max_cells:8192)
show audit.embed_xy_curve(height:0mm).surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
