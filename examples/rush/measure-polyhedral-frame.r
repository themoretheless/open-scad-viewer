// @rush/1
box = brep_box([1,2,3],[3,5,7])
measurement = box.measure_polyhedral_frame(origin_mm:[1,2,3],frame_basis:[[2,0,0],[0,1,0],[0,0,4]],tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
