// @rush/1
wall = corrugated_bellows(center:[0mm,0mm,0mm],mean_radius:8mm,amplitude:2mm,height:24mm,radial_thickness:1mm,periods:6)
show wall.nurbs_patches_tessellate(segments:16)
