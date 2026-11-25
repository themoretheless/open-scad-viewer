// @rush/1
shape = superellipsoid_patches(center:[1mm,2mm,3mm],radii:[4mm,3mm,2mm],exponents:[0.8,1.25],tolerance:2mm,max_cells:2048)
show shape.nurbs_patches_tessellate(segments:16)
