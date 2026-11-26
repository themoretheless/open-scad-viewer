// @rush/1
a = polyline_curve(points:[[-2,-2,0],[2,-2,0],[2,2,0],[-2,2,0]],closed:true)
b = polyline_curve(points:[[-0.5,-0.5,0],[0.5,-0.5,0],[0.5,0.5,0],[-0.5,0.5,0]],closed:true)
result = a.contour_fill_at_point(inputs:[b],point:[0,0],even_odd:false,parameter_tolerance:0.01,max_cells:10000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
