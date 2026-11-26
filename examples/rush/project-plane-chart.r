// @rush/1
panel = bezier_surface(points:[[[0,0,2],[0,2,2]],[[2,0,3],[2,2,3]]])
result = panel.project_plane_chart(max_work:1000000)
show result.tessellate(8,8)
