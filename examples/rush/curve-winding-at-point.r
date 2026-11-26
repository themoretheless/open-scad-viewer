// @rush/1
a = polyline_curve(points:[[-2,-2,0],[2,-2,0],[2,2,0],[-2,2,0]],closed:true)
result = a.curve_winding_at_point(point:[0,0],parameter_tolerance:0.01,max_cells:10000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
