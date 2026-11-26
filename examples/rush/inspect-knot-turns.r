// @rush/1
a = nurbs_curve(degree:1,knots:[0,0,1,3,3],control_points:[[0,0,0],[1mm,0,0],[1mm,2mm,0]],weights:[1,2,1])
result = a.inspect_knot_turns(domain:[0,3],minimum_cosine:0.5,max_knots:1)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
