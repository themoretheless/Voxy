# Model playback event queue

Each ModelPlayback retains per-clip event tracks and a bounded pending event
queue. It previews the matching target track before staging the animator; only
a successful frame consumer publication appends events and advances clocks.
Queue admission uses remaining capacity from a 4096-event budget. Draining
consumes events once. No shared asset owns instance clocks or delivery queues.

219 animation and 172 editor controls (including GPU) pass. New editor control
proves failed consumer publication leaves no events, retry produces its marker
and the second drain is empty. Invalid clip registration rejects.

Registration/drain remain internal and are not yet connected to NodeId-bearing
scene delivery, authored persistence, audio or game callbacks. Source transition
event policy remains unfinished. This is not full production event completion.
