// @rush/1
param depth = 4mm range 1mm..8mm
profile = dovetail_profile(mouth:[0mm,0mm,0mm],mouth_width:4mm,base_width:8mm,depth:depth)
show profile.surface_extrude(vector:[0mm,0mm,12mm]).tessellate(24,4)
