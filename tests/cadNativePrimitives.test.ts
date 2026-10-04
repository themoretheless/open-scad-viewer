import {expect, it} from 'vitest';
import Module, {type Vec2} from '../src/services/geometry/module';

it('keeps signed rectangle sizes, centering and collapsed profiles', async () => {
    const {CrossSection} = await Module();
    for (const size of [[2, 3], [-2, 3], [-2, -3], [0, 3]] as Vec2[]) {
        for (const center of [false, true]) {
            const [x, y] = size;
            const a = center ? -x / 2 : 0, b = center ? -y / 2 : 0;
            const expected = new CrossSection(x === 0 || y === 0 ? [] : [[[a, b], [a + x, b], [a + x, b + y], [a, b + y]]]);
            const actual = CrossSection.square(size, center);
            try {
                expect(actual.toPolygons()).toEqual(expected.toPolygons());
                expect(actual.area()).toBe(expected.area());
                expect(actual.isEmpty()).toBe(expected.isEmpty());
            } finally { actual.delete(); expected.delete(); }
        }
    }
});

it('matches the previous circle sampling for signed radii and fractional segment counts', async () => {
    const {CrossSection} = await Module();
    for (const radius of [2, -2, 0]) {
        for (const segments of [-1, 0, 2, 3, 3.5, 8, 32]) {
            const points = Array.from({length: segments}, (_, i) => [radius * Math.cos(i * 2 * Math.PI / segments), radius * Math.sin(i * 2 * Math.PI / segments)] as Vec2);
            const expected = new CrossSection([points]);
            const actual = CrossSection.circle(radius, segments);
            try {
                expect(actual.area()).toBeCloseTo(expected.area(), 11);
                const a = actual.toPolygons(), b = expected.toPolygons();
                expect(a.map(r => r.length)).toEqual(b.map(r => r.length));
                a.forEach((r, i) => r.forEach((p, j) => p.forEach((v, k) => expect(v).toBeCloseTo(b[i]![j]![k]!, 11))));
                expect(actual.isEmpty()).toBe(expected.isEmpty());
            } finally { actual.delete(); expected.delete(); }
        }
    }
    expect(() => CrossSection.circle(2, 65_537)).toThrow(/budget/i);
});
