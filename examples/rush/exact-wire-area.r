// @rush/1
border = nurbs_curve(degree:1,knots:[3,3,4,5,6,7,7],control_points:[[0,0],[4,0],[4,2],[0,2],[0,0]],weights:[1,2,4,2,1])
audit = border.exact_wire_area(inputs:[],max_work:1000000)
show audit.embed_xy_curve(height:0mm).surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
