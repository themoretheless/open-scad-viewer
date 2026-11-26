// @rush/1
box = brep_box([1,2,3],[3,5,7])
second = brep_box([5,2,3],[6,5,7])
measurement = box.measure_polyhedral_axis_gap(target_geometry:second,direction:[1,0,0],tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
