// @rush/1
path = nurbs_curve(degree:3,knots:[2,2,2,2,6,6,6,6],control_points:[[-3,0],[7,1],[-15,2],[27,3]],weights:[1,1,1,1])
result = path.classify_uv_seam(direction:"u",seam_coordinate:0,max_work:1000000)
show result.embed_xy_curve(height:0mm).surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
