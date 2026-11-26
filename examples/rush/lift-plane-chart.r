// @rush/1
panel = bezier_surface(points:[[[0,0,2],[0,2,2]],[[2,0,3],[2,2,3]]])
edited = bezier_surface(points:[[[4,0,0],[4,2,0]],[[6,0,0],[6,2,0]]])
result = panel.lift_plane_chart(target_geometry:edited,max_work:1000000)
show result.tessellate(8,8)
