// @rush/1
// Discrete RMF with sampled refinement acceptance, not a continuous certificate.
p = circle_curve(center:[0,0,0],normal:[0,0,1],radius:1mm)
c = bezier_curve(points:[[0,0,0],[0,0,8mm]])
show profile_sweep(p,c,normal:[1,0,0],sections:17,max_deviation:0.1mm,scale:{degree:1,knots:[2,2,7,7],values:[1,2],weights:[1,1]}).tessellate(segments_u:24,segments_v:32)
