# Coupled cell-local sensible heat from Maxwell dissipation

InertialBody optionally owns reference-temperature thermal inventory in each
reference tetrahedron. Heat capacity uses its retained cell mass, exactly the
mass used to assemble nodal inertia. The neutral mannequin enables illustrative
cp=3500 J/(kg K) and initial temperature 310.15 K; material and geometry remain
uncalibrated.

Maxwell relaxation deposits released cell energy in this owner. Relative sensible
energy and compensation are canonical; temperature is derived, avoiding an
independent temperature state or loss against a large initial thermal baseline.
A separate deposit defect is included in existing energy admission. Pose,
velocity, material memory, thermal inventory, adaptive predictor and frame clock
publish atomically. Initial-temperature reset is rejected after enabling storage.

The heat receipt is already deposited when thermal storage is enabled. Callers
must not add it again. Spatial conduction, liquid/air exchange and temperature-
dependent material properties remain incomplete.

Validation includes analytic capacity/temperature response, total mechanical plus
thermal energy balance, sub-ulp heat storage, invalid/reset configuration and a
late temperature overflow after mechanical advancement. The full 12-second demo
cycle checks stored cell heat against accumulated receipts. `trace.csv` is
compared with the preceding cycle; no new GPU render is claimed. Qualification,
source hashes and numerical results are in report.json.

The complete engine research/parity/hardware/destruction/fluid objective remains
active. This artifact qualifies only the stated thermal coupling increment.
