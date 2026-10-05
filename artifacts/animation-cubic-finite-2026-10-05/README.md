# Finite cubic vector animation sampling

Position and scale cubic channels retain ordinary finite f32 Hermite evaluation.
If intermediate arithmetic becomes nonfinite, f64 evaluation recovers a finite
representable result. Genuine unrepresentable output remains nonfinite and is
rejected by existing pose/palette admission. Quaternion cubic sampling is unchanged.

Regression independently uses two equal 2e38 endpoints and equal 3e38 tangents
over four seconds: midpoint is 2e38 despite ordinary intermediate overflow.
Opposite incoming tangent produces true overflow, which remains rejected.
Endpoints are exact. All 213 animation tests and all 171 editor tests, including
11 GPU acceptance controls, pass. Zero failures and zero ignored tests. This
does not complete engine parity, hardware coverage or CUDA qualification.
