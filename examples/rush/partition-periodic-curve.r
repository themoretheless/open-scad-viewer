// @rush/1
source = nurbs_curve(degree:1,knots:[-1, 0, 1, 2, 3, 4, 5],control_points:[[0mm,0mm,0mm],[1mm,0mm,0mm],[1mm,1mm,0mm],[0mm,1mm,0mm],[0mm,0mm,0mm]],weights:[1, 1, 1, 1, 1],periodic:true)
result = source.partition_periodic_curve(parameter:3.5,cuts:[4.5,6],part:1,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(32,4)
