// @rush/1
source = bezier_surface(points:[[[0,0,0],[0,1mm,0]],[[1mm,0,0],[1mm,1mm,0]]])
field0 = source.surface_gaussian_field(patch:0,uv:[0,0],return_field:false,max_work:1000000)
field1 = field0.surface_mean_squared_field(patch:0,uv:[0,0],return_field:false,max_work:1000000)
field2 = field1.surface_principal_gap_squared_field(patch:0,uv:[0,0],return_field:false,max_work:1000000)
show field2.tessellate(16,16)
