# Localized draft conflict

Production persist now throws the existing `DRAFT_CONFLICT` code when its
localStorage reference is replaced. The existing localized conflict handler
then displays the direct explanation and next actions, without prepending a
generic storage failure message. Save rejection and geometry preservation
remain unchanged.

Six selected component tests pass, including RU/EN prefix regressions, shared
version Retry and local branch retention. Log:
`/tmp/cad-draft-localized-conflict-20261010.log`. These component tests use mocked
worker/storage boundaries; prior real-browser shared-load acceptance is recorded
separately and predates this message-only fix.
