// @rush/1
box = brep_box([1,2,3],[3,5,7])
measurement = box.measure_section_path_length(plane_normal:[1,0,0],plane_offset_mm:2,query_path_mm:[[2,0,5],[2,6,5],[2,0,5]],max_width_mm:0.000001,tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
