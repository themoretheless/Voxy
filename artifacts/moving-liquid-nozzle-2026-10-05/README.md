# Moving liquid nozzle

The existing PulsedEmitter now adds explicit world source velocity to relative
pulse velocity. Zero default preserves stationary nozzle behavior. Galilean
boost regression checks identical masses/positions and actual momentum shift
M*boost. NaN source velocity and composed velocity overflow preserve particle
state and elapsed emission time. All 3 emitter unit controls and 5 species
integration tests passed in release mode.

This does not integrate source pose, finite-mass reaction or physical breakup.
No GPU preview or broad engine completion is claimed.
