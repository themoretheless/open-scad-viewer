// @rush/1
panel = bezier_surface(points:[[[0,0,2],[0,2,2]],[[2,0,3],[2,2,3]]])
other = bezier_surface(points:[[[4,0,4],[4,2,4]],[[6,0,5],[6,2,5]]])
result = panel.shared_plane_chart(inputs:[other],patch_index:1,max_work:1000000)
show result.tessellate(8,8)
