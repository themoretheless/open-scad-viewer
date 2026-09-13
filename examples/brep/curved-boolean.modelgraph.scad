// @modelgraph-text/1
// Rational cylinder walls survive the Boolean. Tessellation is only display.
a = brep_cylinder(10mm, 20mm)
b = brep_cylinder(10mm, 20mm).transform([[1,0,0,8],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
// Also supported here: brep_union, brep_intersection, brep_xor.
show a.brep_subtract(b).brep_tessellate(16)
