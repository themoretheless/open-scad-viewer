// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
a = chart.partition_exact_periodic_surface_axis(direction:"u",seam:0,cuts:[2],part:0,max_work:1000000).surface_exact_insert_knot(direction:"v",parameter:0.25,count:1,max_work:1000000)
b = chart.partition_exact_periodic_surface_axis(direction:"u",seam:0,cuts:[2],part:1,max_work:1000000).surface_exact_insert_knot(direction:"v",parameter:0.75,count:1,max_work:1000000)
assembled = assemble_periodic_surface_axis_with_exact_knot_union(a,b,direction:"u",max_work:1000000)
show assembled.tessellate(8,8)
