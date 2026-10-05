# Physically accepted fade events

PreparedModelFadeMotion::accepted_playback now previews markers over the receipt's
accepted wall interval and adds them to the candidate queue. The caller retains
the enclosing physics/scene commit boundary. Ordinary ticks and certified fades
share one bounded target-event preview implementation.

Policy: markers are emitted on the target clip timeline only; fading source
markers are not duplicated. This also applies when source and target are the
same clip. Partial receipts use only their accepted interval, not requested dt.

Both existing physical fade regression paths (ordinary compiled and original
sources) now verify that an event exists in a subsequently rejected candidate,
that rollback leaves the current queue empty, and that successful retry drains
one event exactly once. Receipt identity/changed-clock rejection remains tested.

Pending: app/game delivery integration and explicit partial collision marker
boundary regression. No claim of full production animation or engine completion.

Final verification: release editor library suite with normally ignored GPU
controls enabled: 174 passed, zero failed/ignored. git diff --check passed.
Changes remain local and uncommitted.
