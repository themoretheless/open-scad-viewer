// @rush/1
chart = nurbs_surface(degree_u:1,degree_v:1,knots_u:[-1,0,1,2,3,4,5],knots_v:[-1,0,1,2,3,4,5],control_points:[[[0,0,0],[0,1,0],[0,1,1],[0,0,1],[0,0,0]],[[1,0,0],[1,1,0],[1,1,1],[1,0,1],[1,0,0]],[[1,0,1],[1,1,1],[1,1,2],[1,0,2],[1,0,1]],[[0,0,1],[0,1,1],[0,1,2],[0,0,2],[0,0,1]],[[0,0,0],[0,1,0],[0,1,1],[0,0,1],[0,0,0]]],weights:[[1,1,1,1,1],[1,1,1,1,1],[1,1,1,1,1],[1,1,1,1,1],[1,1,1,1,1]],periodic_u:true,periodic_v:true)
patch = chart.restrict_exact_doubly_periodic_surface(u:[3.5,4.5],v:[3.5,4.5],max_work:1000000)
show patch.tessellate(8,8)
