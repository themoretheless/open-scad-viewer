#!/usr/bin/env python3
"""Make an isolated, source-faithful Curvex migration for qualification.

No original source or test is deleted, skipped, or weakened. The dependency
manifest, geometry paths, and gradient sampling boundary are migrated. The
original checkout is read-only. The generated diff is the migration artifact.
"""
from __future__ import annotations
import argparse
import difflib
import hashlib
import json
from pathlib import Path
import re
import shutil

REPLACEMENTS = {
    'kurbo::': 'osv_geometry::curve::',
    'linesweeper::': 'osv_geometry::curve_boolean::',
    'lyon_path::': 'osv_geometry::render::path::',
    'lyon_tessellation::': 'osv_geometry::render::tess::',
}

def migrate_gradient_sampling(source):
    """Make nonlinear colors independent of the tessellator's diagonals."""
    if source.count('enum CacheEntry {') != 1 or source.count('        c.get(key)\n') != 1:
        raise ValueError('Curvex render cache contract changed; review the migration before continuing')
    source = source.replace('enum CacheEntry {',
        'enum CacheEntry {\n    GradientSamples { base: Box<CacheEntry>, fingerprint: u64, sampled: Rc<osv_geometry::attribute_mesh::SampledMesh> },', 1)
    # A colored mesh enriches its existing geometry entry, preserving the
    # original cache_get call contract (capacity is migrated separately).
    source = source.replace('        c.get(key)\n',
        '        c.get(key).map(|entry| match entry {\n'
        '            CacheEntry::GradientSamples { base, .. } => *base,\n'
        '            entry => entry,\n'
        '        })\n', 1)
    # Keep the original tessellation/cache code and all test assertions. Replace
    # only each paint function's per-vertex recolor/remap block.
    for name, next_marker, paint in (
        ('fn paint_gradient_stroke(', '/// Paint the GRADIENT stroke ribbon(s)',
         '    osv_paint_sampled_gradient(painter, &geometry, g, bb, ct, op, revision, sample_key);\n}\n\n'),
        ('fn draw_gradient_mesh(', '#[cfg(test)]\nmod gradient_color_tests',
         '    osv_paint_sampled_gradient(painter, geometry, g, bb, ct, op, revision, Some(sample_key));\n}\n\n'),
    ):
        start = source.index(name)
        end = source.index(next_marker, start)
        function = source[start:end]
        if name == 'fn paint_gradient_stroke(':
            marker = '    let geometry = match cache_key'
            function = function.replace(marker, '    let sample_key = cache_key.clone();\n'+marker, 1)
        else:
            marker = '    let tessellation = match cache_get'
            function = function.replace(marker, '    let sample_key = key.clone();\n'+marker, 1)
        cut = function.index('    let origin = ct.origin;')
        source = source[:start] + function[:cut] + paint + source[end:]
    start = source.index('struct RenderCache {')
    end = source.index('thread_local! {', start)
    source = source[:start] + 'include!("osv_render_cache.rs");\n\n' + source[end:]
    source = source.replace('const RENDER_CACHE_LIMIT: usize = 512;',
                            'const RENDER_CACHE_LIMIT: usize = 8192;', 1)
    return source + '\ninclude!("osv_gradient_sampling.rs");\n'

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('source', type=Path)
    ap.add_argument('destination', type=Path)
    args = ap.parse_args()
    source = args.source.resolve()
    destination = args.destination.resolve()
    kernel = Path(__file__).resolve().parents[2] / 'crates' / 'planar-geometry'
    if destination == source or source in destination.parents:
        ap.error('destination must be outside the original source checkout')
    if destination.exists():
        ap.error('destination already exists; preserve that run and use a new directory')
    if not (source / 'Cargo.toml').is_file():
        ap.error('source must be the Curvex checkout')
    shutil.copytree(source, destination, ignore=shutil.ignore_patterns('.git','target','.DS_Store','.idea','.claude'))
    patch = []
    hashes = {}
    for path in sorted(destination.rglob('*.rs')):
        relative = path.relative_to(destination)
        original = path.read_text()
        hashes[str(relative)] = hashlib.sha256(original.encode()).hexdigest()
        changed = original
        for old, new in REPLACEMENTS.items():
            changed = changed.replace(old, new)
        if str(relative) == 'src/ui/shape_render.rs':
            changed = migrate_gradient_sampling(changed)
        if changed != original:
            path.write_text(changed)
            patch.extend(difflib.unified_diff(original.splitlines(True), changed.splitlines(True),
                         fromfile='a/'+str(relative), tofile='b/'+str(relative)))
    for helper_name in ['gradient_sampling', 'render_cache']:
        helper = Path(__file__).with_name(helper_name+'.rs').read_text()
        helper_path = 'src/ui/osv_'+helper_name+'.rs'
        (destination / helper_path).write_text(helper)
        patch.extend(difflib.unified_diff([], helper.splitlines(True), fromfile='/dev/null', tofile='b/'+helper_path))
    manifest = destination / 'Cargo.toml'
    original = manifest.read_text()
    changed = re.sub(r'(?m)^(kurbo|linesweeper|lyon_path|lyon_tessellation)\s*=.*\n', '', original)
    # JSON escaping is also valid for this TOML basic string.
    changed = changed.replace('[dependencies]\n', '[dependencies]\nosv_geometry = { package = "planar-geometry", path = '+json.dumps(str(kernel))+' }\n', 1)
    manifest.write_text(changed)
    patch.extend(difflib.unified_diff(original.splitlines(True), changed.splitlines(True),fromfile='a/Cargo.toml',tofile='b/Cargo.toml'))
    (destination / 'osv-migration.patch').write_text(''.join(patch))
    (destination / 'osv-migration-source.json').write_text(json.dumps({'source':str(source),'kernel':str(kernel),'sourceHashes':hashes},indent=2)+'\n')
    print(json.dumps({'destination':str(destination),'patch':str(destination/'osv-migration.patch'),'rustFiles':len(hashes)},indent=2))

if __name__ == '__main__':
    main()
