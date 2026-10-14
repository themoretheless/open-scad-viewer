// @rush/1
// x=20u/(1+uv), y=20v/(1+uv), z=12uv/(1+uv); no sampled fitting.
show formula_surface(domain: [0,1,0,1], expressions: [[20,"u","*",1,"u","v","*","+","/"],[20,"v","*",1,"u","v","*","+","/"],[12,"u","*","v","*",1,"u","v","*","+","/"]]).tessellate(segments_u: 16, segments_v: 32)
