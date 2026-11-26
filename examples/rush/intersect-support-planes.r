// @rush/1
left = bezier_surface(points:[[[0,0,0],[0,1,1]],[[1,0,1],[1,1,2]]])
right = bezier_surface(points:[[[0,0,1],[0,1,0]],[[1,0,3],[1,1,2]]])
audit = left.intersect_support_planes(target_geometry:right,max_work:1000000)
show audit.tessellate(8,8)
