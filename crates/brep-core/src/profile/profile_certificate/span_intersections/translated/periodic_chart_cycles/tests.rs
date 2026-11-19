use super::*;
fn touching() -> (Vec<OriginalPeriodicChartCycleEdge>, Vec<Vec<Port>>) {
    let endpoints = [
        [0, 1],
        [1, 2],
        [2, 3],
        [3, 0],
        [0, 4],
        [4, 5],
        [5, 6],
        [6, 0],
    ];
    let directions = [
        [1, 0],
        [0, 1],
        [-1, 0],
        [0, -1],
        [-1, 0],
        [0, -1],
        [1, 0],
        [0, 1],
    ];
    let mut ports = vec![Vec::new(); 7];
    let edges = endpoints
        .iter()
        .enumerate()
        .map(|(i, &vertices)| {
            for end in 0..2 {
                let s = directions[i].map(|n| if end == 0 { n } else { -n });
                ports[vertices[end]].push(Port {
                    edge: i,
                    end,
                    sector: sector(s).unwrap(),
                    rank: None,
                });
            }
            OriginalPeriodicChartCycleEdge {
                vertices,
                kind: OriginalPeriodicChartCycleEdgeKind::SourceTrace(i),
            }
        })
        .collect();
    (edges, ports)
}
#[test]
fn exact_clockwise_successors_keep_two_touching_material_fans_separate() {
    let (edges, mut ports) = touching();
    let (successor, walks) = permutations(&edges, &mut ports, 8).unwrap();
    assert_eq!(walks, vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]]);
    assert_eq!(successor[3], 0);
    assert_eq!(successor[7], 4);
    assert_eq!(ports[0].len(), 4);
}
#[test]
fn incomplete_or_duplicated_ports_and_walk_exhaustion_never_publish_partial_cycles() {
    let (edges, mut ports) = touching();
    assert!(permutations(&edges, &mut ports, 7).is_err());
    let (_, mut ports) = touching();
    ports[0].pop();
    assert!(permutations(&edges, &mut ports, 8).is_err());
    let (_, mut ports) = touching();
    let duplicate = ports[0][0].clone();
    ports[0].push(duplicate);
    assert!(permutations(&edges, &mut ports, 8).is_err());
}
