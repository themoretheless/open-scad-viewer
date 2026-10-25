    use super::*;
    #[test]
    fn revision_exhaustion_never_wraps_or_publishes_candidate() {
        let snapshot = TopologySnapshot::new(sheet(), |point| {
            require(point.iter().all(|x| x.is_finite()), "Invalid point")
        })
        .unwrap();
        let mut store = TopologyStore::new(snapshot);
        store.revision = u64::MAX;
        let old_tolerance = store.snapshot().model().tolerance_mm;
        let edit = store
            .checkout()
            .prepare(|m| {
                m.tolerance_mm = 2e-6;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            store.commit(edit).unwrap_err().code,
            "BREP_REVISION_EXHAUSTED"
        );
        assert_eq!(store.revision(), u64::MAX);
        assert_eq!(store.snapshot().model().tolerance_mm, old_tolerance);
    }
    #[test]
    fn history_is_bounded_and_branching_invalidates_redo() {
        let snapshot = TopologySnapshot::new(sheet(), |_| Ok(())).unwrap();
        let mut store = TopologyStore::new(snapshot);
        assert!(!store.undo().unwrap());
        assert!(!store.redo().unwrap());
        assert_eq!(store.revision(), 0);
        for i in 0..40 {
            let tx = store
                .checkout()
                .prepare(|m| {
                    m.tolerance_mm = (i + 1) as f64 * 1e-6;
                    Ok(())
                })
                .unwrap();
            store.commit(tx).unwrap();
        }
        assert_eq!(store.history_lengths(), (MAX_TOPOLOGY_HISTORY, 0));
        for _ in 0..MAX_TOPOLOGY_HISTORY {
            assert!(store.undo().unwrap());
        }
        assert!(!store.undo().unwrap());
        assert_eq!(store.history_lengths(), (0, MAX_TOPOLOGY_HISTORY));
        assert_eq!(store.snapshot().model().tolerance_mm, 8e-6);
        assert!(store.redo().unwrap());
        let tx = store
            .checkout()
            .prepare(|m| {
                m.tolerance_mm = 5e-6;
                Ok(())
            })
            .unwrap();
        store.commit(tx).unwrap();
        assert!(!store.redo().unwrap());
        assert_eq!(store.history_lengths(), (2, 0));
        store.revision = u64::MAX;
        assert!(store.undo().is_err());
        assert_eq!(store.history_lengths(), (2, 0));
        assert_eq!(store.snapshot().model().tolerance_mm, 5e-6);
    }
    #[test]
    fn restored_history_cannot_change_owner_admission_policy() {
        let original = TopologySnapshot::new(sheet(), |_| Ok(())).unwrap();
        let foreign = TopologySnapshot::new(sheet(), |_| Ok(())).unwrap();
        assert!(TopologyStore::from_history(original.clone(), vec![foreign], vec![]).is_err());
        assert!(TopologyStore::from_history(original.clone(), vec![original], vec![]).is_ok());
    }
    #[test]
    fn prepared_state_cannot_substitute_its_admission_policy() {
        #[derive(Clone)]
        struct OwnerState {
            value: u32,
            policy: u32,
        }
        impl RevisionState for OwnerState {
            type Model = u32;
            fn model(&self) -> &u32 {
                &self.value
            }
            fn same_admission_policy(&self, other: &Self) -> bool {
                self.policy == other.policy
            }
            fn try_edit(&self, edit: impl FnOnce(&mut u32) -> Result<()>) -> Result<Self> {
                let mut next = self.clone();
                edit(&mut next.value)?;
                next.policy += 1; // Deliberately faulty owner implementation.
                Ok(next)
            }
        }
        let store = RevisionStore::new(OwnerState {
            value: 7,
            policy: 1,
        });
        let result = store.checkout().prepare(|v| {
            *v = 9;
            Ok(())
        });
        assert!(matches!(
            result,
            Err(Error {
                code: "BREP_TRANSACTION_POLICY_MISMATCH",
                ..
            })
        ));
        assert_eq!(*store.snapshot().model(), 7);
        assert_eq!(store.revision(), 0);
        assert_eq!(store.history_lengths(), (0, 0));
    }
    fn sheet() -> Model<(), (), ()> {
        Model {
            vertices: vec![
                Vertex {
                    point: [0., 0., 0.],
                },
                Vertex {
                    point: [1., 0., 0.],
                },
                Vertex {
                    point: [0., 1., 0.],
                },
            ],
            edges: vec![
                Edge {
                    degenerate: false,
                    vertices: [0, 1],
                    curve: (),
                },
                Edge {
                    degenerate: false,
                    vertices: [1, 2],
                    curve: (),
                },
                Edge {
                    degenerate: false,
                    vertices: [2, 0],
                    curve: (),
                },
            ],
            loops: vec![Loop {
                coedges: (0..3)
                    .map(|edge| Coedge::new(
                        edge,
                        false,
                        (),
                    ))
                    .collect(),
            }],
            faces: vec![Face {
                surface: (),
                outer: 0,
                holes: vec![],
            }],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: 1e-7,
        }
    }
    #[test]
    fn no_geometry_kernel_required() {
        sheet().validate_topology().unwrap();
    }
    #[test]
    fn coedge_trim_is_validated_and_preserves_authored_direction() {
        let coedge = Coedge::new(3, true, ());
        assert_eq!(coedge.trim(), None);
        let trim = CoedgeTrim {
            curve_parameter: [2., 1.],
            pcurve_parameter: [-1., 4.],
            periodic_lift: [[1, 0], [-2, 3]],
        };
        let trimmed = coedge.with_trim(trim).unwrap();
        assert!(trimmed.reversed);
        assert_eq!(trimmed.trim(), Some(trim));

        let invalid = CoedgeTrim {
            curve_parameter: [0., 0.],
            ..trim
        };
        assert!(Coedge::new(3, false, ()).with_trim(invalid).is_err());
    }
    #[test]
    #[cfg(feature = "codec")]
    fn legacy_coedge_without_trim_remains_readable() {
        let legacy = Coedge::new(7, true, ());
        let value = value_codec::Serialize::to_value(&legacy);
        assert!(value.get("trim").is_none());
        let restored: Coedge<()> = value_codec::Deserialize::from_value(value).unwrap();
        assert_eq!(restored, legacy);
        assert_eq!(restored.trim(), None);
    }
    #[test]
    #[cfg(feature = "codec")]
    fn coedge_trim_round_trips_through_codec() {
        let trim = CoedgeTrim {
            curve_parameter: [3., -2.],
            pcurve_parameter: [0.25, 0.75],
            periodic_lift: [[0, 1], [-1, 0]],
        };
        let coedge = Coedge::new(9, true, ()).with_trim(trim).unwrap();
        let value = value_codec::Serialize::to_value(&coedge);
        assert!(value.get("trim").is_some());
        let restored: Coedge<()> = value_codec::Deserialize::from_value(value).unwrap();
        assert_eq!(restored, coedge);
        assert_eq!(restored.trim(), Some(trim));
    }
    #[test]
    #[cfg(feature = "codec")]
    fn legacy_tolerance_migrates_to_versioned_context() {
        let model = sheet();
        let mut value = value_codec::Serialize::to_value(&model);
        value.as_object_mut().unwrap().remove("toleranceContext");
        let restored: Model<(), (), ()> = value_codec::Deserialize::from_value(value).unwrap();
        let context = restored.tolerance_context().unwrap();
        assert_eq!(context.spatial_bounds().on_mm, model.tolerance_mm);

        let canonical = value_codec::Serialize::to_value(&restored);
        assert!(canonical.get("toleranceMm").is_some());
        assert!(canonical.get("toleranceContext").is_some());
    }
    #[test]
    #[cfg(feature = "codec")]
    fn conflicting_legacy_and_context_tolerances_are_rejected() {
        let mut value = value_codec::Serialize::to_value(&sheet());
        *value.get_mut("toleranceMm").unwrap() = value_codec::Serialize::to_value(&2e-6_f64);
        assert!(<Model<(), (), ()> as value_codec::Deserialize>::from_value(value).is_err());
    }
    #[test]
    fn closed_shell_requires_two_opposite_uses() {
        let mut m = sheet();
        m.shells[0].closed = true;
        assert!(m.validate_topology().is_err());
    }
    #[test]
    fn bodies_cannot_reference_open_shells() {
        let mut m = sheet();
        m.bodies.push(Body {
            outer_shell: 0,
            inner_shells: vec![],
        });
        assert!(m.validate_topology().is_err());
    }
    #[test]
    fn broken_reference_is_a_typed_error() {
        let mut m = sheet();
        m.loops[0].coedges[1].edge = 99;
        assert_eq!(
            m.validate_topology().unwrap_err().code,
            "BREP_INVALID_TOPOLOGY"
        );
    }

    #[test]
    fn explorer_euler_characteristic_genus_and_adjacency() {
        let m = sheet();
        // Triangle sheet: V=3, E=3, F=1 => chi = 3 - 3 + 1 = 1.
        assert_eq!(m.euler_characteristic(), 1);
        assert_eq!(m.shell_euler_characteristic(0), Some(1));
        assert_eq!(m.shell_genus(0), None); // open shell
        assert_eq!(m.vertex_edges(0), vec![0, 2]);
        assert_eq!(m.face_edges(0), vec![0, 1, 2]);
        assert!(m.adjacent_faces(0).is_empty());

        // Closed tetrahedron: V=4, E=6, F=4 => chi = 2, genus = 0.
        let tet = Model {
            vertices: vec![
                Vertex { point: [0., 0., 0.] },
                Vertex { point: [1., 0., 0.] },
                Vertex { point: [0., 1., 0.] },
                Vertex { point: [0., 0., 1.] },
            ],
            edges: vec![
                Edge { degenerate: false, vertices: [0, 1], curve: () },
                Edge { degenerate: false, vertices: [1, 2], curve: () },
                Edge { degenerate: false, vertices: [2, 0], curve: () },
                Edge { degenerate: false, vertices: [0, 3], curve: () },
                Edge { degenerate: false, vertices: [1, 3], curve: () },
                Edge { degenerate: false, vertices: [2, 3], curve: () },
            ],
            loops: vec![
                Loop {
                    coedges: vec![
                        Coedge::new(0, true, ()),
                        Coedge::new(2, true, ()),
                        Coedge::new(1, true, ()),
                    ],
                },
                Loop {
                    coedges: vec![
                        Coedge::new(0, false, ()),
                        Coedge::new(4, false, ()),
                        Coedge::new(3, true, ()),
                    ],
                },
                Loop {
                    coedges: vec![
                        Coedge::new(1, false, ()),
                        Coedge::new(5, false, ()),
                        Coedge::new(4, true, ()),
                    ],
                },
                Loop {
                    coedges: vec![
                        Coedge::new(2, false, ()),
                        Coedge::new(3, false, ()),
                        Coedge::new(5, true, ()),
                    ],
                },
            ],
            faces: (0..4)
                .map(|i| Face {
                    surface: (),
                    outer: i,
                    holes: vec![],
                })
                .collect(),
            shells: vec![Shell {
                faces: (0..4)
                    .map(|face| FaceUse {
                        face,
                        reversed: false,
                    })
                    .collect(),
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: 1e-7,
        };
        tet.validate_topology().unwrap();
        assert_eq!(tet.euler_characteristic(), 2);
        assert_eq!(tet.shell_euler_characteristic(0), Some(2));
        assert_eq!(tet.shell_genus(0), Some(0));
        assert_eq!(tet.genus(), 0);
        assert_eq!(tet.adjacent_faces(0), vec![1, 2, 3]);
    }
