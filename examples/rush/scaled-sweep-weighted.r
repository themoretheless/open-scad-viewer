// @rush/1
// Fixed orientation; independently normalized rational path and positive scale.
p = circle_curve(center:[0,0,0],normal:[0,0,1],radius:1mm)
c = bezier_curve(points:[[0,0,0],[3mm,2mm,4mm],[0,0,8mm]],weights:[1,2,1])
show scaled_sweep(p,c,origin:[0,0,0],scale:{degree:1,knots:[10,10,14,14],values:[1,2],weights:[3,1]}).tessellate(segments_u:24,segments_v:32)
