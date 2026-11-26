// @rush/1
box = brep_box([1,2,3],[3,5,7])
measurement = box.bound_polyhedral_area(max_width_mm2:0.00000001,tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
