// @rush/1
// Circle chart x=10(1-t^2)/(1+t^2), y=20t/(1+t^2), z=0, t in [0,1].
profile = formula_curve(domain: [0,1], expressions: [[10,1,"t","t","*","-","*",1,"t","t","*","+","/"],[20,"t","*",1,"t","t","*","+","/"],[0]])
show profile.surface_extrude([0mm,0mm,12mm]).tessellate(segments_u: 16, segments_v: 32)
