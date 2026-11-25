// @rush/1
profile = cst_section_integrals(chord_mm:2mm,n1:1,n2:1,upper_coefficients:[0.25,0.25],lower_coefficients:[-0.25,-0.25],max_work:1000000)
show profile.surface_extrude(vector:[0,0,1mm]).tessellate(32,4)
