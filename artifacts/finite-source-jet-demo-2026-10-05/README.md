# Finite-source jet demonstration

Existing liquid_jet_film entrypoint now accepts fixed/finite source mode as its
third argument. Finite source uses the transactional mass-changing emitter and
explicit point-source drift. Capture, bounce and spray finite modes all exit 0;
fixed capture also exits 0. Full source/fluid/substrate momentum accounts external
fluid gravity and has maximum selected residual 6.08e-18 kg m/s.

The source releases 1 g from 10 g initial mass and reaches 0.105918 m/s upward
recoil. Capture deposits 0.9 g in film; spray creates 72 numerical fragments.
These are CPU controls, not physical breakup convergence or whole-scene energy
qualification. No new GPU visualization is claimed.
