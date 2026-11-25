// @rush/1
wave = cubic_wave_surface(origin:[0mm,0mm,2mm],width:24mm,length:8mm,amplitude:3mm,periods:2)
show wave.tessellate(segments_u:32,segments_v:4)
