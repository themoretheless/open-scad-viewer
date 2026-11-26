// @rush/1
box = brep_box([1,2,3],[3,5,7])
other = brep_box([2,2,3],[4,5,7])
measurement = box.clip_union_path(inputs:[other],query_path_mm:[[0,"7/2",5],[5,"7/2",5]],tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
