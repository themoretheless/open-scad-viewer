// @rush/1
param ring_radius = 3mm range 2mm..8mm
ring = exact_ring_torus(center:[0,0,0],axes:[[1,0,0],[0,1,0],[0,0,1]],major_radius:ring_radius,minor_radius:1mm,max_work:1000000)
show ring.tessellate(24,12)
