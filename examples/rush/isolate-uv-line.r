// @rush/1
path = nurbs_curve(degree:2,knots:[2,2,2,6,6,6],control_points:[[1,0],[1,1],[0,1]],weights:[1,1,2])
result = path.isolate_uv_line(line_coefficients:[3,4,-5],parameter_width:0.0000001,max_work:1000000)
show result.embed_xy_curve(height:0mm).surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
