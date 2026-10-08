// @rush/1
u0 = bezier_curve(points: [[0mm,0mm,0mm],[0.5mm,0mm,0mm],[1mm,0mm,0mm]])
u1 = bezier_curve(points: [[0mm,0.5mm,0mm],[0.5mm,0.5mm,0.375mm],[1mm,0.5mm,0.5mm]])
u2 = bezier_curve(points: [[0mm,1mm,0mm],[0.5mm,1mm,1mm],[1mm,1mm,1mm]])
v0 = bezier_curve(points: [[0mm,0mm,0mm],[0mm,0.5mm,0mm],[0mm,1mm,0mm]])
v1 = bezier_curve(points: [[0.5mm,0mm,0mm],[0.5mm,0.5mm,0.25mm],[0.5mm,1mm,0.75mm]])
v2 = bezier_curve(points: [[1mm,0mm,0mm],[1mm,0.5mm,0.5mm],[1mm,1mm,1mm]])
show gordon_surface(u_curves: [u0,u1,u2],v_curves: [v0,v1,v2],parameters_u: [0,0.5,1],parameters_v: [0,0.5,1]).tessellate(segments_u: 24,segments_v: 24)
