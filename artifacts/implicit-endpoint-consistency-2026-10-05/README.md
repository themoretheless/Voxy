# Endpoint reconstruction comparison

Read-only rejected-step probes compared the path endpoint used by the nonlinear solve against the endpoint reconstructed from its impulse velocity. On the imported step-65 leaf, maximum differences were 8.44e-15 to 2.54e-14 m. The corresponding endpoint potential changes ranged around +/-1e-6 J and matched the dominant previously recorded work rejection. Final comparison: -2.9991769e-6 J endpoint potential change versus -2.9943217e-6 J work defect, with 3.0517578125e-10 J admission budget. Uniform-acceleration potential difference for these tiny endpoint shifts is negligible at that scale.

The experimental displacement_velocity_defect_j column in this initial trace omitted acceleration potential and is not a valid complete energy defect. It was not used for admission or the fix. The endpoint-position and raw potential-difference comparisons remain valid. Current tracing uses full diagnostics including acceleration potential.

An independent open-contact fixture shows that a single world-coordinate ULP at a very small positive gap changes barrier energy by more than 1e-7 J while both endpoints remain collision-admissible.
