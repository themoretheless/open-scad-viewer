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
correct ownership of a source edit. Parentage of sector faces needs explicit
qualification before this preview can become a user command.
The reconstruction subdivides original annular caps and side charts, so source
face ownership must describe splits rather than pretend all faces persisted.
