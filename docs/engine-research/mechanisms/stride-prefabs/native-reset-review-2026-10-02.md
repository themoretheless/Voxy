# Native numeric prefab Reset review

The macOS editor was reviewed through its native window with a one-model prefab.
The source angular speed was 1, and the instance member override was 2.

Observed in the native window: the Behavior inspector displayed the difference
marker and Reset control; clicking Reset changed speed to 1 and removed both;
undo restored speed 2 and both controls. Redo, F5 save and F9 load completed;
the saved scene retained instance `outer` and contained an empty override map.
The before and reset scene files are in the temporary review fixture at
`/tmp/voxy-prefab-reset-review-2026-10-02`.

That review found the fixed-width button clipped its text to `Rese`. The button
now measures the label's glyph advances, rounds the width up, adds horizontal
padding, and reserves separate field and hit-test space. A fresh application
build visibly displayed the complete `Reset` label next to speed 2.
The reviewed fresh binary SHA-256 was
`c6396804193c6d38ffc0cc027f3045e83504c3d6e191c27df2a756b936ef8d42`.

After the layout change, 71 editor library tests passed, with two GPU tests
ignored; the application build and package boundary check passed. Native actions
after the final label observation were rejected by the automation tool because
the user changed the app; they are not counted as a second completed save/load
review. Screenshots were viewed through the UI tool, not archived as files.

Scope remains numeric inspector targets and this angular-motion fixture. Generic
component controls, stable collection-item addressing and broad prefab visual
acceptance remain open. During the earlier launch the surface was occluded and
input could change editor state before a new frame became visible; presentation
gating for editor panel actions remains a separate issue.
