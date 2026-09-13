# G0 v3 / G1 v19 source re-freeze

This amendment updates source fingerprints after the SVG, BRep and related work
changed manifests, locks, notices and previously bound source files. The G1
candidate bundle includes `openscadSemanticLowerer.ts`; the pinned legacy support
bundle includes `examples.ts`. A matching fingerprint does not demonstrate that
their behavior matches the oracle.

The transition preserves G0 fingerprint v1/v2 and every G1 plan v1 through v18 as
exact archived bytes. G1 v19 retains the same 65 oracle cases, 18 comparator
mutations, finite boundary matrix, environments, seeds, resource limits, required
clean runs and **4740 work units**. Oracle, reference implementations, schema and
environment bindings cannot be changed by this refresh script.

G1 remains unqualified: `qualificationClaim: none`, approval remains
`not-approved`, u07 remains open, and G0 remains open. The new candidate starts
with **zero** completed clean runs and work units. Every previous result remains
discovery-only. No qualification result or execution approval is generated.

Use the read-only preparation command while sources are changing:

```sh
node scripts/refresh-qualification-fingerprints.mjs --check
```

After the bound files settle, explicitly publish the next versions:

```sh
node scripts/refresh-qualification-fingerprints.mjs --write \
  --recorded-at YYYY-MM-DD \
  --reason 'Describe the actual dependency, notice and source changes requiring this new freeze.'
```

The script records the observed `rustc --version`, hashes ordinary LF files,
rechecks the complete input snapshot during publication, and creates only new
G0 v3, G1 v19 and re-freeze status files. It refuses historical-byte changes,
oracle/schema/environment drift, source races and existing output files.
The status file binds the newly created artifacts and all 20 historical files;
its zero counters describe the new candidate and do not replace an execution log.

`run-g1-candidate-clean-fragment.mjs` and
`run-g1-ubuntu-docker-fragment.mjs` now select v19 and its new result/fragment
paths. Their execution and classification rules are unchanged; existing v18
output cannot count toward v19. Future clean runs still require the exact frozen toolchains, fresh OS jobs,
clean installation, all matrix rows and append-only evidence prescribed by the
plan; passing the fingerprint tests satisfies none of those execution steps.
