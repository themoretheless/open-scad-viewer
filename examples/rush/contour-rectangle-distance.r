// @rush/1
a = polyline_curve(points:[[-2,-2,0],[2,-2,0],[2,2,0],[-2,2,0]],closed:true)
b = polyline_curve(points:[[-0.5,-0.5,0],[0.5,-0.5,0],[0.5,0.5,0],[-0.5,0.5,0]],closed:true)
result = a.contour_rectangle_distance(inputs:[b],center:[0,0],axes:[[0,1],[-1,0]],half_extents:[0.125,0.25],distance_tolerance_mm:0.001,max_cells:10000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
