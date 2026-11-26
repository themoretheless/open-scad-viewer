// @rush/1
a = polyline_curve(points:[[-2,-2,0],[2,-2,0],[2,2,0],[-2,2,0]],closed:true)
b = polyline_curve(points:[[-0.5,-0.5,0],[0.5,-0.5,0],[0.5,0.5,0],[-0.5,0.5,0]],closed:true)
c = polyline_curve(points:[[3,-1,0],[5,-1,0],[5,1,0],[3,1,0]],closed:true)
result = a.planar_contour_sets_distance(inputs:[b,c],first_count:2,distance_tolerance_mm:0.001,max_cells:10000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
