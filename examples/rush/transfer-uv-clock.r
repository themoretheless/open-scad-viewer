// @rush/1
path = nurbs_curve(degree:1,knots:[3,3,4,5,5],control_points:[[2,-3],[7,2],[2,-3]],weights:[1,1,1])
piece = path.pullback_uv_clock(uv_domain:[[2,7],[-3,2]],clock_factor:[2,0.5],piece:0,max_work:1000000)
show piece.embed_xy_curve(height:0mm).surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
