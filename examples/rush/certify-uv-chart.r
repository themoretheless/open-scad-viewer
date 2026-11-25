// @rush/1
path = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0,0],[0.5,2],[1,0]],weights:[1,1,1])
proof = path.certify_uv_chart(uv_domain:[[0,1],[0,1]],max_work:1000000)
show proof.embed_xy_curve(height:0mm).surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
