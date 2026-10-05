# Interactive finite-source mode

Existing liquids entrypoint accepts --finite-source, with normal pause/reset/exit
controls. SceneApp exposes a finite-source builder. Restart preserves the mode.
Native smoke verification checks source mass depletion, positive recoil, finite
energy reserve and fluid mass/spreading. Final smoke exits 0 with 120 presented
frames and 242 fixed steps on Apple M4 Max / Metal. This does not qualify CUDA,
other graphics backends, calibrated nozzle flow or rotating source mechanics.
