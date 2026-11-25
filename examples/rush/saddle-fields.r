// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,1mm,0]],[[1mm,0,0],[1mm,1mm,1mm]]])
du = panel.surface_partial_field(direction:"u",patch:0,uv:[0.5,0.25],return_field:false,max_work:1000000)
normal = du.surface_area_normal_field(patch:0,uv:[0.5,0.25],return_field:false,max_work:1000000)
metric = normal.surface_metric_field(patch:0,uv:[0.5,0.25],return_field:false,max_work:1000000)
mixed = metric.surface_mixed_partial_field(orders:[1,1],patch:0,uv:[0.5,0.25],return_field:false,max_work:1000000)
show mixed.tessellate(12,12)
