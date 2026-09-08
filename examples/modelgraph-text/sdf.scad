// @modelgraph-text/1
// Smooth union of two implicit spheres; mesh is sampled within explicit bounds.
a = sdf_sphere([-5mm,0,0], 10mm)
b = sdf_sphere([5mm,0,0], 10mm)
show sdf_smooth_union(a,b,radius:3mm).sdf_tessellate([-20mm,-20mm,-20mm],[20mm,20mm,20mm],[24,24,24])
