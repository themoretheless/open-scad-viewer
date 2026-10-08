// @rush/1
// x=10u, y=10v, z=4(u^3-3uv^2); power-basis conversion without sampled fit.
show formula_surface(domain: [-1,1,-1,1], expressions: [[10,"u","*"],[10,"v","*"],[4,"u","u","*","u","*",3,"u","*","v","*","v","*","-","*"]]).tessellate(segments_u: 16, segments_v: 32)
