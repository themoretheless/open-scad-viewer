// @rush/1
panel = bezier_surface(points:[[[0mm,0mm,0mm],[0mm,8mm,0mm]],[[10mm,0mm,0mm],[10mm,8mm,4mm]]])
left = panel.surface_split(direction:"v",parameter:0.25,part:0)
right = panel.surface_split(direction:"v",parameter:0.25,part:1)
joined = left.merge_patch_strip(inputs:[right],direction:"v",max_work:1000000)
show joined.tessellate(8,8)
