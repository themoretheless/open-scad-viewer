// @rush/1
box = brep_box([1,2,3],[3,5,7])
measurement = box.clip_section_segment(plane_normal:[1,0,0],plane_offset_mm:2,query_path_mm:[[2,0,5],[2,6,5]],tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
