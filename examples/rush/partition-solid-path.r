// @rush/1
box = brep_box([1,2,3],[3,5,7])
measurement = box.partition_solid_path(query_path_mm:[[0,"7/2",5],[2,"7/2",5],[4,"7/2",5]],tolerance_uv:0.00000001,max_work:1000000)
show measurement.brep_tessellate(4)
