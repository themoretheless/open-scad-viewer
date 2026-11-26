// @rush/1
box = brep_box([1,2,3],[3,5,7])
measurement = box.section_polyhedral_faces(plane_normal:[1,0,0],plane_offset_mm:2,tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
