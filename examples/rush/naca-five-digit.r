// @rush/1
profile = naca_five_digit_profile(profile_code:23012,chord_mm:2mm,closed:true,samples:128,degree:3,control_count:32)
show profile.surface_extrude(vector:[0,0,1mm]).tessellate(64,4)
