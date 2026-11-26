// @rush/1
profile = line_curve(start:[-1mm,0mm,0mm],end:[1mm,0mm,0mm])
spring = profile.profile_spring_surface(center:[0mm,0mm,0mm],radius:8mm,height:12mm,turns:1,phase_degrees:90deg,tolerance_mm:0.001mm,max_spans:64)
show spring.nurbs_patches_tessellate(segments:8)
