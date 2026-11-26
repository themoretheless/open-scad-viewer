// @rush/1
path = nurbs_curve(degree:1,knots:[2,2,6,6],control_points:[[0,0],[10,10]],weights:[1,1])
result = path.transfer_uv_grid_piece(old_u_breaks:[0,5,10],old_v_breaks:[0,5,10],u_breaks:[10,20,40],v_breaks:[-9,1,11],u_factors:[2,0.5],v_factors:[0.5,2],piece:1,max_work:1000000)
show result.embed_xy_curve(height:0mm).surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
