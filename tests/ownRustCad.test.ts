import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import Module from '../src/services/geometry/module';
import { compileModelGraphText } from '../src/services/modelGraphText';
import { parseOpenSCAD } from '../src/services/openscadParser';
describe('own Rust CAD qualification v1', () => {
    it('preserves analytic volumes and closed boundaries in overlapping and nested CSG', async () => {
        for (const [source, volume] of [
            ['union(){cube(2);translate([1,0,0])cube(2);}', 12],
            ['intersection(){cube(2);translate([1,0,0])cube(2);}', 4],
            ['difference(){cube(2);translate([1,0,0])cube(2);}', 4],
            ['difference(){cube(4);translate([1,1,1])cube(2);}', 56],
            ['linear_extrude(3)difference(){square(4);translate([1,1])square(2);}', 36],
        ] as const) {
            const result = await parseOpenSCAD(source);
            expect(result.volume, source).toBeCloseTo(volume, 7);
            for (const mesh of result.meshes) {
                expect(mesh.topology.boundary, source).toBe(0);
                expect(mesh.topology.nonManifold, source).toBe(0);
            }
        }
    });
    it('retains concavity in Minkowski sums', async () => {
        const wasm = await Module();
        const profile = new wasm.CrossSection([[0, 0], [2, 0], [2, 1], [1, 1], [1, 2], [0, 2]]);
        const left = profile.extrude(1), right = wasm.Manifold.cube(1);
        const sum = left.minkowskiSum(right);
        // Expanded L footprint: 3*3 minus its 1*1 missing corner, height 2.
        expect(sum.volume()).toBeCloseTo(16, 6);
        for (const handle of [profile, left, right, sum])
            handle.delete();
    });
    it('supports offset erosion, projection, slicing and plane splitting', async () => {
        const wasm = await Module();
        const profile = new wasm.CrossSection([[0, 0], [4, 0], [4, 4], [0, 4]]);
        const eroded = profile.offset(-1, 'Miter');
        expect(eroded.area()).toBeCloseTo(4, 8);
        const mesh = profile.extrude(3);
        const projected = mesh.project(), sliced = mesh.slice(1);
        expect(projected.area()).toBeCloseTo(16, 8);
        expect(sliced.area()).toBeCloseTo(16, 8);
        const halves = mesh.splitByPlane([1, 0, 0], 2);
        expect(halves.map(m => m.volume())).toEqual([24, 24]);
        for (const handle of [profile, eroded, mesh, projected, sliced, ...halves])
            handle.delete();
        expect(() => mesh.volume()).toThrow(/deleted/i);
    });
    it('builds the complete SKADIS box without open boundaries', async () => {
        const source = readFileSync(new URL('../examples/skadis-box/skadis-dovetail.modelgraph.scad', import.meta.url), 'utf8');
        const result = await parseOpenSCAD(compileModelGraphText(source).source);
        expect(result.meshes).toHaveLength(3);
        // Manufacturing-scale tolerance against the previous independently recorded box.
        expect(Math.abs(result.volume - 160352.6604181734)).toBeLessThan(10);
        for (const mesh of result.meshes) {
            expect(mesh.topology.boundary).toBe(0);
            expect(mesh.topology.nonManifold).toBe(0);
        }
    });
});
