import {expect, it} from 'vitest';
import Module from '../src/services/geometry/module';

it('preserves Euler order, identity and closed winding through native solid transforms', async () => {
    const {CadSolid} = await Module();
    const source = CadSolid.cube([2, 3, 5]);
    const first = source.rotate([37, 0, 0]);
    const second = first.rotate([0, -23, 0]);
    const expected = second.rotate([0, 0, 71]);
    const actual = source.rotate([37, -23, 71]);
    const reflected = actual.mirror([2, 3, 4]);
    const restored = reflected.mirror([2e200, 3e200, 4e200]);
    const empty = source.mirror([0, 0, 0]);
    try {
        for (const key of ['min', 'max'] as const) {
            actual.boundingBox()[key].forEach((v, i) => expect(v).toBeCloseTo(expected.boundingBox()[key][i], 11));
            restored.boundingBox()[key].forEach((v, i) => expect(v).toBeCloseTo(actual.boundingBox()[key][i], 11));
        }
        expect(actual.originalID()).toBe(source.originalID());
        expect(reflected.volume()).toBeCloseTo(30, 10);
        expect(reflected.status()).toBe('NoError');
        expect(empty.isEmpty()).toBe(true);
        expect(source.boundingBox()).toEqual({min: [0, 0, 0], max: [2, 3, 5]});
    } finally {
        [source, first, second, expected, actual, reflected, restored, empty].forEach(v => v.delete());
    }
});

it('uses native rotation and reflection for planar profiles without reversing their area', async () => {
    const {CrossSection} = await Module();
    const source = CrossSection.square([2, 3]);
    const rotated = source.rotate(90);
    const reflected = source.mirror([1, 0]);
    const empty = source.mirror([0, 0]);
    try {
        expect(rotated.area()).toBeCloseTo(6, 12);
        expect(reflected.area()).toBeCloseTo(6, 12);
        const points = rotated.toPolygons().flat();
        expect(Math.min(...points.map(p => p[0]))).toBeCloseTo(-3, 12);
        expect(Math.max(...points.map(p => p[1]))).toBeCloseTo(2, 12);
        expect(empty.isEmpty()).toBe(true);
        expect(source.area()).toBeCloseTo(6, 12);
    } finally {
        [source, rotated, reflected, empty].forEach(v => v.delete());
    }
});

it('preserves affine matrix layout, negative scaling and collapsed axes in native transforms', async () => {
    const {CadSolid, CrossSection} = await Module();
    const source = CadSolid.cube([2, 3, 5]);
    const scaled = source.scale([-2, 3, 4]);
    const shifted = scaled.translate([7, -11, 13]);
    const matrix = source.transform([-2, 0, 0, 99, 0, 3, 0, 98, 0, 0, 4, 97, 7, -11, 13, 96]);
    const collapsed = source.scale([0, 1, 1]);
    const section = CrossSection.square([2, 3]);
    const planar = section.scale([-2, 3]).translate([7, -11]);
    const planarMatrix = section.transform([-2, 0, 99, 0, 3, 98, 7, -11, 97]);
    const zero = section.scale(0);
    try {
        expect(shifted.boundingBox()).toEqual({min: [3, -11, 13], max: [7, -2, 33]});
        expect(matrix.boundingBox()).toEqual(shifted.boundingBox());
        expect(matrix.volume()).toBeCloseTo(720, 10);
        expect(matrix.status()).toBe('NoError');
        expect(matrix.originalID()).toBe(source.originalID());
        expect(collapsed.boundingBox()).toEqual({min: [0, 0, 0], max: [0, 3, 5]});
        expect(collapsed.volume()).toBe(0);
        expect(planarMatrix.bounds()).toEqual(planar.bounds());
        expect(planarMatrix.area()).toBeCloseTo(36, 12);
        expect(zero.area()).toBe(0);
        expect(source.volume()).toBeCloseTo(30, 12);
    } finally {
        [source, scaled, shifted, matrix, collapsed, section, planar, planarMatrix, zero].forEach(v => v.delete());
    }
});
