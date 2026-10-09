import {expect, it} from 'vitest';
import {createBrepGraphBuilder, mirrorMatrix, rotationMatrix, scaleMatrix, translationMatrix} from '../src/services/solid/brepGraph';

it('builds graph matrices through the shared native affine API', () => {
    expect(translationMatrix(2, -3, 5)).toEqual([[1, 0, 0, 2], [0, 1, 0, -3], [0, 0, 1, 5], [0, 0, 0, 1]]);
    expect(scaleMatrix(-2, 0, 4)).toEqual([[-2, 0, 0, 0], [0, 0, 0, 0], [0, 0, 4, 0], [0, 0, 0, 1]]);
    expect(mirrorMatrix(0, 0, 0)).toBeNull();
    const apply = (m: number[][], p: number[]) => m.slice(0, 3).map(row => row.slice(0, 3).reduce((s, v, i) => s + v * p[i]!, row[3]!));
    const source = [2, -3, 5];
    const expected = apply(rotationMatrix(0, 0, 71), apply(rotationMatrix(0, -23, 0), apply(rotationMatrix(37, 0, 0), source)));
    apply(rotationMatrix(37, -23, 71), source).forEach((v, i) => expect(v).toBeCloseTo(expected[i]!, 12));
    const m = mirrorMatrix(2e200, 3e200, 4e200)!;
    apply(m, apply(m, source)).forEach((v, i) => expect(v).toBeCloseTo(source[i]!, 12));
});

it('keeps graph IDs and rational quadratic data for four native circle quadrants', () => {
    for (const radius of [5, -5, 0]) {
        const graph = createBrepGraphBuilder();
        const ids = graph.circleLoop(radius);
        expect(ids).toEqual(['n0', 'n1', 'n2', 'n3']);
        graph.nodes.forEach((node, i) => {
            expect(node).toMatchObject({op: 'curve', degree: 2, knots: [0, 0, 0, 1, 1, 1], periodic: false});
            const points = node.control_points as number[][];
            const weights = node.weights as number[];
            weights.forEach((w, j) => expect(w).toBeCloseTo(j === 1 ? Math.SQRT1_2 : 1, 15));
            for (let j = 0; j < 3; j++) {
                const angle = (i + j / 2) * Math.PI / 2;
                const r = radius / weights[j]!;
                expect(points[j]![0]).toBeCloseTo(r * Math.cos(angle), 12);
                expect(points[j]![1]).toBeCloseTo(r * Math.sin(angle), 12);
                expect(points[j]![2]).toBe(0);
            }
        });
    }
});

it('keeps authored rectangle corners, IDs and collapsed edges through the native constructor', () => {
    for (const size of [[4, 6], [-4, 6], [0, 6], [0, 0]] as [number, number][]) {
        for (const center of [false, true]) {
            const graph = createBrepGraphBuilder();
            expect(graph.rectangleLoop(size, center)).toEqual(['n0', 'n1', 'n2', 'n3']);
            const [x, y] = size;
            const [a, b] = center ? [-x / 2, -y / 2] : [0, 0];
            const corners = [[a, b, 0], [a + x, b, 0], [a + x, b + y, 0], [a, b + y, 0]];
            graph.nodes.forEach((node, index) => {
                expect(node).toMatchObject({op: 'curve', degree: 1, knots: [0, 0, 1, 1], control_points: [corners[index], corners[(index + 1) % 4]]});
            });
        }
    }
});
