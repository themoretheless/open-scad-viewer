// @rush/1
box = brep_box([1,2,3],[3,5,7])
measurement = box.bound_swept_first_contact(query_path_mm:[[-2,"7/2",5],[2,"7/2",5]],query_radius_mm:"1/2",max_parameter_width:0.00001,tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
