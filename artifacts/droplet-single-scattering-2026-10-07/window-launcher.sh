#!/bin/sh
env VOXY_TEMPORAL_EXTINCTION=1 VOXY_TEMPORAL_SCATTERING=1 '/tmp/Voxy Scattering.app/Contents/MacOS/engine' > /Users/themoretheless/Documents/ChatGPT/Voxy/artifacts/droplet-single-scattering-2026-10-07/window-runtime.log 2>&1
voxy_exit=$?
printf "%s\n" "$voxy_exit" > /Users/themoretheless/Documents/ChatGPT/Voxy/artifacts/droplet-single-scattering-2026-10-07/window-exit.txt
exit "$voxy_exit"
