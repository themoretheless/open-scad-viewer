"""Consume the two snapshots from export-blender-qualification.mts in real Blender."""
import importlib.util
import json
import sys
from pathlib import Path
import bpy
root = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('solid_bridge', root/'integrations/blender/solid_bridge.py')
bridge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bridge)
directory = Path(sys.argv[sys.argv.index('--') + 1])
bridge.register()
from_browser='--browser-downloads' in sys.argv
filename_prefix='browser' if from_browser else 'width'
project_id=json.loads((directory/f'{filename_prefix}-2.osv-blender.json').read_text())['projectId']
resuming = '--resume' in sys.argv
if resuming:
    bpy.ops.wm.open_mainfile(filepath=str(directory/'bridge-scene.blend'), load_ui=False)
    external = bpy.data.objects['Keep unrelated object']
else:
    external = bpy.data.objects.new('Keep unrelated object', None)
    bpy.context.scene.collection.objects.link(external)

def load(width):
    assert bpy.ops.import_scene.solid_snapshot(filepath=str(directory/f'{filename_prefix}-{width}.osv-blender.json')) == {'FINISHED'}
    return {o[bridge.BODY]: o for o in bpy.data.objects if o.get(bridge.PROJECT) == project_id}

def bounds(obj):
    return [[min(v.co[i] for v in obj.data.vertices),max(v.co[i] for v in obj.data.vertices)] for i in range(3)]

if resuming:
    saved = {o[bridge.BODY]:o for o in bpy.data.objects if o.get(bridge.PROJECT)==project_id}
    assert len(saved)==2 and bounds(saved['source'])==[[0,7],[0,3],[0,4]]
    assert saved['source'].name=='User name' and saved['source'].location[:]==(4.,5.,6.)
    assert saved['source'].modifiers[0].name=='User bevel'
    assert saved['source'].data.materials[0].name=='User material'
    assert saved['source'].parent==external
    assert saved['source']['artist_note']=='retain after reopening'
    saved_pointers={key:obj.as_pointer() for key,obj in saved.items()}
objects = load(2)
if resuming:
    assert {key:obj.as_pointer() for key,obj in objects.items()}==saved_pointers

assert set(objects) == {'source','instance'}
assert bounds(objects['source']) == [[0,2],[0,3],[0,4]]
assert bounds(objects['instance']) == [[10,12],[0,3],[0,4]]
bpy.context.view_layer.update()
assert abs(objects['source'].dimensions.x - .002) < 1e-8
pointers = {key: obj.as_pointer() for key,obj in objects.items()}
source = objects['source']
source.parent = external
source['artist_note']='retain after reopening'
source.location = (4,5,6)
source.name = 'User name'
modifier = source.modifiers.get('User bevel') or source.modifiers.new('User bevel','BEVEL')
modifier.width = .1
material = bpy.data.materials.get('User material') or bpy.data.materials.new('User material')
if not source.data.materials: source.data.materials.append(material)
objects = load(7)
assert {key: obj.as_pointer() for key,obj in objects.items()} == pointers
assert bounds(source) == [[0,7],[0,3],[0,4]]
assert bounds(objects['instance']) == [[10,17],[0,3],[0,4]]
assert source.location[:] == (4.,5.,6.) and source.name == 'User name'
assert source.modifiers[0] == modifier and source.data.materials[0] == material
assert external.name in bpy.data.objects
bpy.context.view_layer.update()
assert abs(objects['instance'].dimensions.x - .007) < 1e-8
report = {'blender': bpy.app.version_string, 'build': bpy.app.build_hash.decode(),
          'source_bounds_mm': bounds(source), 'instance_bounds_mm': bounds(objects['instance']),
          'instance_width_m': objects['instance'].dimensions.x,
          'object_identity_preserved': True, 'user_transform_preserved': True,
          'modifier_material_name_preserved': True, 'unrelated_object_preserved': True}
assert len([o for o in bpy.data.objects if o.get(bridge.PROJECT)==project_id])==2
assert source.parent==external and source['artist_note']=='retain after reopening'
report['parent_and_custom_property_preserved']=True
report['reopened_in_new_process']=resuming
report['bridge_object_count']=2
report['input_from_browser_download']=from_browser
if not resuming:
    bpy.ops.wm.save_as_mainfile(filepath=str(directory/'bridge-scene.blend'),check_existing=False)
name='verification-browser-download.json' if from_browser else 'verification-reopened.json' if resuming else 'verification.json'
(directory/name).write_text(json.dumps(report,indent=2)+'\n')
bridge.unregister()
print('BLENDER_ROUNDTRIP_VERIFIED: '+json.dumps(report))
