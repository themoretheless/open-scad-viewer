# Source identity retention

The partial-annular-source-identities native example enumerates all source
edges of tube(20,5,6). Four source arcs admit preview construction. For each:

- All 16 original vertices remain with identical coordinates and identities.
- Eight original edge identities remain.
- The original body identity remains with explicit Persisted ownership.
- No original face identity remains.
- The result's persistent names are complete and ChangeSet validates.

The focused source preview test now asserts exact original vertex identity
retention, eight retained edge identities, and original body identity plus
an explicit body Persisted record for every admitted arc. Serialization and
reload must retain that body identity and complete names.
The source model remains unchanged. The report includes the full ChangeSet.

Complete generated naming and a structurally valid ChangeSet do not establish
correct ownership of a source edit. The four tested arcs now have explicit
source parentage for all 24 retained-support sector faces; three blend faces
are generated. Each retained-support face has exactly one source parent and
all ten source faces are represented. Ambiguous support ownership is refused.
The native test verifies these relations as well as ChangeSet validity.
The reconstruction subdivides original annular caps and side charts, so source
face ownership must describe splits rather than pretend all faces persisted.
