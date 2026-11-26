// @rush/1
box = brep_box([1,2,3],[3,5,7])
measurement = box.locate_section_point(plane_normal:[1,0,0],plane_offset_mm:2,query_point_mm:[2,"7/2",5],tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
