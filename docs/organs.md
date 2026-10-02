# Internal-organ mechanics: current implementation and remaining scope

## Anatomical geometry

Twenty-two female Human Reference Atlas subassemblies are exported in `assets/anatomy/hra-female`. `tools/export_female_organs.py` is a reproducible standard-library-only extractor; it preserves connectivity, original world coordinates, semantic structure identifiers and provenance. `cargo run --release -p voxy_app --example organs_render` verifies actual importer/GPU rendering. See the asset README for exact coverage and topology diagnostics.

The meshes are anatomical surfaces and overlapping regional annotations. They are not directly usable as closed volumetric FEM organs. They have not been registered inside the Blender character. Skeleton coverage is partial; anal sphincter geometry is missing.

## Orthotropic myocardium

`physics::biomechanics::Myocardium` implements a Holzapfel–Ogden-type passive orthotropic energy, with an explicit volumetric penalty:

- `J = det(F) > 0`, `Cbar = J^(-2/3) F^T F`.
- Matrix: `a/(2b) [exp(b (I1bar - 3)) - 1]`.
- Fiber and sheet: `ai/(2bi) [exp(bi positive(I4ibar - 1)^2) - 1]`.
- Fiber–sheet interaction: `afs/(2bfs) [exp(bfs I8fsbar^2) - 1]`.
- Volume: `K/2 (J - 1)^2`.
- Optional nominal active fiber tension: `activation T0 (|F f0| - 1)`.

Fiber/sheet directions must form an orthonormal material frame. All scales are in pascals and energy is per reference volume. First Piola stress is the analytic derivative of the complete energy, including the isochoric normalization. Invalid material frames, inversion and numerical overflow are rejected.

Source for the orthotropic constitutive structure: Holzapfel and Ogden (2009), [Constitutive modelling of passive myocardium](https://pubmed.ncbi.nlm.nih.gov/19657007/). This implementation uses an isochoric/penalty adaptation. No universal default coefficients are provided; the numerical tests use synthetic parameters, not measured human calibration.

`Body::set_myocardium(element_index, law)` assigns the law and local frame to individual tetrahedra. Both assembled forces and spatial stress diagnostics use the assigned law. Existing `set_activation` and cavity pressure remain available. The equilibrium solver is quasistatic; iteration count is not physiological time. The optional active term is not an electrophysiology, calcium or length-dependent contractility model.

Verification covers constitutive finite differences under mixed deformation, activation and shear; objectivity of energy/stress; rest stress; directional stiffness; rejection of invalid inputs; FEM energy/force assembly; spatial stress conversion; and equilibrium contraction under activation. These checks verify implementation, not cardiac anatomy or experimental realism.

## Time-dependent soft-organ material and FEM coupling

`ViscoelasticOgden::new(terms, bulk_pa, branches)` provides a multi-term equilibrium Ogden potential plus reference-strain Maxwell branches. Terms accept positive or negative nonzero exponents. No material constants are labeled as measured liver/brain properties.

For eigenvalues `lambda_i²` of `Cbar`, the equilibrium potential is
`sum_terms 2 mu / alpha² [sum_i lambda_i^alpha - 3] + K/2 (J-1)²`.
The `alpha=2` case reduces to the existing isochoric neo-Hookean law. Tensor powers are evaluated using a symmetric eigendecomposition with a reconstruction check; first Piola stress is analytic and remains well-defined at repeated eigenvalues.

Maxwell branches use the objective reference strain `E = dev(Cbar)/2` and branch potential `mu_v ||E-Q||²`. An implicit Euler update gives
`Q_new = w Q_old + (1-w) E_new`, `w = tau/(tau+dt)`.
Eliminating Q and its viscous dissipation potential produces the incremental branch energy `mu_v w ||E_new-Q_old||²`. This is a deliberate reference-strain formulation; it is **not** the multiplicative finite-viscoelastic law fitted in [Budday et al. (2017)](https://pubmed.ncbi.nlm.nih.gov/28756040/). That study motivates the need for nonlinear, regional and time-dependent brain mechanics; its fitted constants have not been transferred to this different formulation. Liver viscoelasticity also needs organ-specific experiments; see [Nonlinear viscoelastic constitutive model for bovine liver tissue](https://pmc.ncbi.nlm.nih.gov/articles/PMC7502455/).

`Body::set_viscoelastic_ogden(index, law)` assigns this law per tetrahedron. `Body::relax_step(dt_seconds, iterations, force_tolerance)` advances real relaxation time while solving quasistatic force equilibrium. It freezes old memory throughout all nonlinear iterations, then commits all branch updates once after convergence. Invalid input, inversion, nonconvergence or update overflow leaves the entire body unchanged. Force assembly and spatial-stress diagnostics use the same material response. Assigning this passive law rejects nonzero muscle activation.

`ViscoelasticOgden::advance(F, dt)` can also run prescribed-deformation experiments. It reports nonnegative backward-Euler viscous loss per reference volume. Held-strain tests verify the discrete Maxwell relaxation curve and stored-energy loss bound; load-controlled FEM tests verify creep and atomic failure. Mixed-deformation finite differences, the neo-Hookean limit and rotation tests verify the spectral constitutive derivatives.

Reproduce a synthetic two-branch relaxation/creep experiment:

```sh
cargo run --release -p physics --example organ_relaxation > /tmp/voxy-organ-relaxation.csv
```

The CSV records actual seconds, held-strain shear stress, load-controlled tip displacement, viscous loss and force residual. It is a numerical specimen, not an anatomical organ. The current linear tetrahedra and bulk penalty can lock near incompressibility; a mixed displacement-pressure discretization, inertia, volume anatomy, contact and experimental calibration remain necessary for the full requested realism.

## Airway network and lung-unit recoil

`physics::respiration` adds a resistive airway graph, storage-free junctions and compliant terminal units. Volume flow follows `Q = (P_from-P_to)/R`; `Airway::poiseuille` computes laminar rigid-pipe resistance from length, radius and viscosity in SI units. Terminal units have either linear compliance or logarithmic recoil `Ptp = P0 + a ln(V/V0)`. `Ptp = P_alveolar-P_pleural`; pleural and mouth pressures are prescribed inputs.

`RespiratoryNetwork::step` enforces backward-Euler terminal volume balance and Kirchhoff flow balance at every junction. The nonlinear pressure solve uses Newton iterations, a positive-definite Jacobian, Cholesky factorization and a line search. Invalid input, nonpositive volumes, numerical failure or nonconvergence leaves all network state unchanged. It reports mouth-delivered volume, regional volume change, nodal residual and airway resistive loss `sum R Q² dt`.

This is a lumped incompressible volume-flow model, without gas inertia/compressibility, gas exchange, surface tension, recruitment or airway wall dynamics. It is not yet coupled to the anatomical lung surfaces or a 3D tissue volume. The exported transpulmonary pressure is an interface input for that future coupling. The [Berger et al. lung model](https://arxiv.org/abs/1411.1491) motivates the eventual coupling of airway flow to continuum parenchyma; this implementation does not reproduce that poroelastic model.

Six tests verify the exact discrete single-compartment RC solution, volume conservation, inspiratory/expiratory flow reversal, heterogeneous regional filling, nonlinear cyclic ventilation, pressure-gauge invariance and atomic failure. Run the synthetic two-region breathing experiment with:

```sh
cargo run --release -p physics --example respiration > /tmp/voxy-respiration.csv
```

## Closed-loop circulation

`physics::circulation` implements a closed incompressible blood-volume graph. Every compartment follows `P = P_external + E(t) (V-V0)` with prescribed time-varying elastance. Connections obey the backward-Euler momentum relation
`deltaP = R Q + B |Q| Q + L (Q_new-Q_old)/dt`.
Ideal check valves constrain Q to be nonnegative and can sustain a reverse pressure difference when closed. With nonzero inertance they may retain forward flow during deceleration; they close when the effective driving pressure reaches zero. There is no valve leaflet dynamics or leakage model.

`Vessel::rigid_pipe` derives R from laminar Poiseuille flow, L from plug-flow inertance and B from an optional local-loss coefficient, using supplied length, radius, viscosity and density. The fluid is Newtonian with constant properties; hematocrit, non-Newtonian shear thinning, microvascular cell effects and vessel-wall pulsation are not yet included.

`Circulation::step` solves nodal volume conservation with a coupled Newton/Cholesky solve. The derivative of inertial/quadratic flow and each active valve enters the Jacobian. Positive compartment volume is enforced by line search. Failure preserves all volumes, pressures and flow history. Diagnostics include volume drift, residual, resistive loss `dt sum (R Q²+B |Q|³)` and kinetic energy `sum L Q²/2`.

Tests verify exact discrete closed RC equilibration, total blood-volume conservation, energy dissipation for passive flow, valve directionality, discrete inertial momentum balance, pressure-gauge invariance, pulsatile pumping, geometric pipe scaling and atomic rejection. Reproduce the synthetic four-chamber/systemic/pulmonary circuit with:

```sh
cargo run --release -p physics --example circulation > /tmp/voxy-circulation.csv
```

The example supplies explicit synthetic parameters and prescribed atrial/ventricular activation. It is not fitted human physiology. Its twelve-second run uses 6000 steps and a 5000 ml blood volume; measured drift across the run was below 3.5e-11 ml. The chamber and regional pressure/volume outputs are available for later coupling; they are not currently registered to the anatomical vessel/heart geometry or computed from 3D myocardial deformation.

The general architecture is informed by [closed-loop lumped cardiovascular mechanics](https://pmc.ncbi.nlm.nih.gov/articles/PMC3751725/) and the need for [3D electromechanics/circulation coupling](https://arxiv.org/abs/2011.15040). The current implementation does not reproduce those full models or their fitted parameters. A linear volume law also lacks venous collapse and pericardial interaction. No oxygen transport, lymph circulation or neural control is claimed.

## Coupled deformable cavity and blood flow

`biomechanics::FemChamber` maps one actual FEM cavity onto a circulation compartment. `Body::cavity_volume` integrates the oriented deformed boundary using translation-relative coordinates. At each trial transmural pressure the wall is solved to force equilibrium, and the resulting volume/compliance overrides the circuit's nominal elastance law. Compliance is measured with a configurable finite pressure increment, using central differences where positive wall pressure permits them.

The outer pressure Newton solve enforces blood-volume balance while the inner FEM solves account for the pressure-dependent wall deformation. Every trial uses the same old constitutive history and loads; pressure-tangent solves may warm-start geometry from the central-pressure equilibrium without copying its updated viscous memory; only the final accepted wall state and circuit are committed together. Subsequent circuit steps warm-start from accepted coupled pressure, rather than inferring pressure from a nominal linear elastance. Invalid initial volume correspondence, unstable/nonpositive compliance, failed force equilibrium or failed outer convergence rejects the whole step.

`Circulation::step_with_volume_response` is the constitutive interface for selected nonlinear compartments. Its callback receives index and trial transmural pressure and returns volume plus positive compliance. Callback-owned side effects are outside the circuit transaction, so the FEM adapter evaluates only local clones. `Circulation::elastic_energy` now returns an error after custom-law steps: the nominal elastance potential must not be used in place of the actual wall potential.

Run a small active-wall specimen with inlet/outlet check valves and two reservoirs:

```sh
cargo run --release -p physics --example fem_circulation > /tmp/voxy-fem-circulation.csv
```

Tests verify geometric cavity volume equals circuit volume, pressure actually changes the wall mesh, active contraction ejects blood, and failed steps preserve both geometry and fluid state. This is a synthetic annular-wall specimen, not the atlas heart geometry. Current limitations are quasistatic wall mechanics, one independent cavity per body, finite-difference compliance, nonnegative transmural pressure and no shared-septum cross-compliance. Full four-chamber 3D electromechanics, tissue inertia/contact and anatomical wall-volume reconstruction remain required.

Low-pressure verification exposed cancellation in the near-rest neo-Hookean energy. It now uses the exact identity `tr(D)=-I2(D)-det(D)` for `D=Cbar-I` near rest, and `expm1` for fiber recruitment energy. A tiny-shear regression test checks the energy down to shear 1e-8; the pressure specimen at 1 Pa now reaches the same 1e-8 N residual tolerance without exhausting the iteration budget or weakening the force tolerance. Force derivatives and existing material checks still pass.

## Expanded anatomy and scope

The atlas exporter additionally supplies mammary glands, vagina, uterus, ovaries, fallopian tubes and supporting reproductive ligaments, plus eyes, optic nerves, spinal cord, upper/lower tooth sets, regional blood vasculature, lymphatic organs, eye/knee muscles and selected adipose regions. Primitive anatomy IDs and geometry audits remain in the manifest. `cargo run --release -p voxy_app --example organs_render -- --all` imports and renders all 22 groups. Individual names may also be passed to inspect selected groups, e.g. `-- eyes teeth`.

These are incomplete reference surfaces. The lymphatic-organ group contains spleen, thymus and a reference lymph node, not a body-wide lymph-vessel network. Vascular geometry is regional, without measured hydraulic properties or a validated graph. Eye and knee muscles do not cover all muscles. Fat covers selected abdominal subcutaneous and visceral regions. Optic nerves and spinal cord do not cover the peripheral nervous system. Tooth sets are outer geometry, not resolved enamel/dentin/pulp layers. Blood and lymph are fluid states represented by numerical networks, not anatomical surface meshes; those networks are not yet registered to this atlas.

## Still required for the full objective

- Validated tetrahedral heart walls/cavities, measured regional fiber/sheet fields, contractility and circulation coupling.
- Lung parenchyma, airway pressure/flow network, air–tissue interaction, pleura and diaphragm mechanics.
- Layered bowel/sphincter walls with active muscle waves, lumen transport and contact.
- Liver and brain calibrated viscoelasticity, organ attachments, brain/skull/CSF interaction.
- Full skeleton, joints and bone constitutive properties, skin/organ/bone coupling.
- Vascular graph and pressure/flow coupling, arterial/venous wall mechanics, blood rheology and circulation, lymphatic valves and transport.
- Eye compartments and cornea/sclera mechanics, layered teeth and periodontal mechanics, neural signaling and anatomical nerve coverage.
- Regional fat mechanics, full muscle anatomy and activation/force-length-velocity mechanics, fascia/tendon attachment.
- Breast-specific layered fat/gland/skin mechanics and suspensory ligaments; genital and reproductive organ wall mechanics, perfusion, attachments and calibration.
- Full organ and skin self/mutual contact, friction, geometry registration, calibration and validation against experimental observations.

No whole-body or clinically validated simulation is claimed by the current assets and laws.

### Numerical safeguards for the coupled solve

Ogden equilibrium energy now sums `exp(z)-1-z` in normalized principal-log stretches, with a small-argument series. This prevents near-rest spectral energy from vanishing through cancellation. The test suite checks small shear, energy/stress derivatives and the neo-Hookean limit.

When an equilibrium trial lies within a square-root-machine-epsilon displacement scale and its energy change lies inside a stiffness-scaled floating-point bound, the solver can accept it only if the separately evaluated force residual strictly decreases. Ordinary steps still use Armijo energy descent. The requested force convergence tolerance is unchanged. This safeguard resolves rounding-limited active-wall steps rather than reporting unconverged geometry as accepted.

A coupled viscous-wall test compares accepted branch stresses to a separate single accepted-pressure increment. This checks that repeated volume/compliance probes have not advanced memory multiple times. The cyclic valve/reservoir example also passes the full 0.02–0.24 s activation cycle with actual wall and fluid volumes matching.


## Interstitial fluid and lymph protein transport

`physics::lymph::LymphNetwork` adds a separate conservative fluid/protein network. Nodes can represent plasma, interstitial fluid and lymph. Pressure follows an explicitly supplied linear compliance law, and oncotic pressure is `coefficient * protein_mass / fluid_volume`. Each exchange edge uses:

- `Q = hydraulic_conductance * (Pfrom - Pto + pump_head - reflection * (oncotic_from - oncotic_to))`.
- `Jprotein = (1-reflection) * Q * donor_concentration + protein_permeability * (Cfrom-Cto)`.
- Each volume/protein transfer is subtracted from its donor and added to its recipient. One-way valve edges clamp reverse hydraulic drive and close both transport paths when closed.

The capillary law is a classical Starling approximation motivated by [Himeno et al., interstitial volume regulation](https://pmc.ncbi.nlm.nih.gov/articles/PMC5381436/). The diffusive/convective structure is informed by [Pietribiasi et al., fluid and protein transport](https://pmc.ncbi.nlm.nih.gov/articles/PMC4970790/). Donor concentration is a positive conservative transport approximation; this is not that paper's multi-pore mean-concentration formula. No published fitted coefficients have been copied.

Integration is explicit with a user-supplied maximum temporal step and additional limits on outgoing fluid/protein to preserve positivity. It is first order: reducing the maximum step is required for accuracy assessment. Errors, overflow and exceeding 100,000 substeps leave the complete prior state unchanged. Reports include integrated edge transfers and drift of both conserved totals.

`cargo run --release -p physics --example lymph_exchange` emits a synthetic 600-second open-versus-impaired-drainage comparison as CSV. Tests independently verify the exact two-compartment hydraulic and protein-diffusion limits, temporal refinement, oncotic balance, protein reflection, one-way closure/pump head, conservation around a plasma–tissue–lymph loop, positivity and failure rollback.

This is not a revised glycocalyx Starling model: there is no subglycocalyx concentration, resolved endothelial layer, nonlinear tissue-compliance safety factor or lymphangion contraction law. Protein is a single mixture, not albumin/globulin species. Plasma storage belongs to this exchange network and is not yet coupled to `Circulation`'s blood volume. Ordinary nodes use the supplied linear compliance; `PoreTissue` supplies mechanical pressure feedback for one uniformly pressurized FEM tissue node. The example is a numerical specimen, not a clinical edema prediction or a calibrated normal lymph flow.

Measured example result (600 s, maximum step 0.1 s): tissue volume is 1000.721757761 mL with open drainage and 1001.670071632 mL with 1% conductance. Maximum total-volume drift is 2.17e-17 m³ and protein-mass drift 1.06e-15 kg. Halving the step to 0.05 s changes sampled tissue volume by at most 0.00134 mL (open) and 0.00180 mL (impaired). These measurements characterize this synthetic calculation only.


## Porous tissue swelling and fluid-network feedback

`Body::set_pore_fluid(PoreFluid)` assigns a single uniform pore-fluid store to a tetrahedral body. Let `V0` be reference tissue volume, `V` its actual deformed tetrahedral volume, `Vf0` reference fluid volume, `Vf` fluid inventory, `alpha` the Biot coefficient and `S>0` total fixed-geometry storage in m³/Pa. The additional potential is

`Efluid = [Vf-Vf0-alpha*(V-V0)]²/(2*S)`, with `p = [Vf-Vf0-alpha*(V-V0)]/S`.

Differentiating this potential adds `-alpha*p*J*F^(-T)` to the first Piola stress and `-alpha*p*I` to the total Cauchy stress reported by `Body::stresses_at`. The storage balance is `Vf-Vf0 = alpha*(V-V0)+S*p`. The FEM preconditioner includes the undrained storage stiffness; omitting it caused oscillatory slow convergence in the retained-fluid specimen. The skeleton energy and its material properties are unchanged.

This is a finite-volume, uniform-pressure extension of Biot storage, not a full spatial biphasic solver. The distinction between solid effective stress and pore-fluid pressure follows the [FEBio biphasic theory](https://febiosoftware.github.io/febio-docs/theory/chapter2/2.7-biphasic-material/); finite storage and constant alpha here do not reproduce FEBio's intrinsically incompressible-mixture formulation. Fluid inventory is a reference-density-equivalent volume, not a separately resolved geometric pore mesh. There are no fitted tissue porosity/storage coefficients. Negative pressure is numerically supported without cavitation or desaturation physics.

`PoreTissue::step` maps one fluid-network node to this storage. At every explicit transport substep, a converged elastic FEM equilibrium supplies the node pressure; the other nodes retain their linear laws. A final equilibrium matches the accepted fluid volume exactly. Actual accepted pressures are retained by `LymphNetwork::pressures` and used by its flux diagnostics. Body geometry and the complete network commit together only after success. Initial fluid inventories must match. History-dependent viscoelastic laws are rejected by this adapter until a joint temporal-history update is implemented.

`LymphNetwork::step_with_pressure_law` is the constitutive callback used by the adapter. External callback effects are not automatically transactional; the supplied FEM adapter uses private clones. Calling ordinary `step` subsequently selects the linear pressure laws again.

`cargo run --release -p physics --example fem_lymph` emits a synthetic closed plasma–porous-tissue–lymph loop. Tests check energy/force finite differences, total Cauchy pressure contribution, storage balance, retained-fluid swelling, two-way flow/deformation feedback, volume/protein conservation and joint rollback on failed FEM or invalid pressure callbacks.

Remaining for the uniform store: general spatial Darcy flow and permeability tensors (cell-resolved scalar interfaces are described below), multiple coupled tissue regions, nonlinear storage and saturation, porous contact/boundary drainage, linkage to the cardiovascular circuit, viscosity/inertia, and experimental calibration on registered anatomical volume meshes. The current uniform-pressure specimen is not a complete edema or organ perfusion model.

Measured coupled example (20 s, maximum substep 0.02 s): final pore pressure 19.950433055 Pa, tissue volume 1.669014234365e-7 m³ (reference 1.666666666667e-7 m³). Maximum total-fluid drift is 1.70e-21 m³; protein-mass drift is 3.05e-20 kg. Reducing the substep to 0.01 s changes sampled pressure by at most 0.00760 Pa and tissue volume by 8.94e-14 m³. These are numerical verification measurements of a synthetic specimen, not human tissue calibration.


## Cell-resolved pressure and spatial Darcy interfaces

`Body::set_cell_pore_fluids` replaces the uniform store with one independent fluid inventory and Biot storage per tetrahedron. Each cell uses its own actual deformed volume in the storage potential. The assembled energy sums all cell potentials; forces and total Cauchy stress include each cell's local pressure. This permits heterogeneous fluid contents, storage and Biot coefficients. A body selects either uniform or cell-resolved storage, never both. Cell-pressure getters reject the uniform mode and vice versa.

`CellPoreTissue` maps these stores to distinct fluid-network nodes. At every explicit transport increment, local inventories drive a shared nonlinear FEM equilibrium; the resulting vector of local pressures drives intercellular transfer. Final inventories, pressures and geometry commit together. Failure preserves every store and fluid/protein state. Like the uniform adapter, this currently requires elastic materials rather than time-dependent Maxwell histories.

`Exchange::darcy` constructs an isotropic interface conductance `G = A / [mu*(d0/k0+d1/k1)]`: the two half-cell resistances are in series, so discontinuous permeability uses the correct harmonic resistance. `Q=G*(p0-p1)`; protein follows conservative donor advection. Zero permeability on either side blocks the interface. There is no osmotic membrane reflection or check valve on this Darcy edge.

`Body::reference_darcy_interface` obtains area and normal half-cell distances from an actual shared tetrahedral face. Returned node IDs are cell IDs and require a matching network mapping. The geometry is fixed in the reference configuration. It checks that both centers are normal to the face and straddle it; nonorthogonal cells are rejected. A two-point flux is inconsistent on general meshes, as explained by the [DuMux TPFA documentation](https://dumux.org/docs/doxygen/master/group___c_c_tpfa_discretization.html) and [Nordbotten's finite-volume hydromechanical study](https://pmc.ncbi.nlm.nih.gov/articles/PMC4280486/). This is therefore a supported orthogonal-interface discretization, not arbitrary anatomical-mesh Darcy flow. MPFA/mixed flow and a deformed permeability push-forward remain necessary for general anatomy and large strains.

`cargo run --release -p physics --example cell_fem_darcy` demonstrates pressure redistribution and local elastic swelling in two adjacent synthetic tetrahedra, emitting CSV. Verification includes local pore-energy finite differences, spatial total stress, interface geometry and hydraulic scaling, heterogeneous series resistance, impermeability, nonorthogonal-grid rejection, exact discrete two-cell diffusion, convergence to the analytical continuous decay, two-way deformation/pressure feedback, conserved fluid/protein inventories and failure rollback.

This is additional numerical infrastructure, not a calibrated skin/organ perfusion model. Anatomical tetrahedralization and tissue registration, permeability measurements, capillary/lymph graph reconstruction, species transport, saturation and general pressure discretization are still required.

Measured two-cell example (10 s, maximum substep 0.01 s): after 0.1 s the cell pressures are 48.970414761 and 2.533477049 Pa; after 10 s they are 25.755284322 and 25.753645931 Pa. Both cells deform locally and approach equal fluid content. At the 100 saved sample times, total-fluid and protein-mass differences from their initial values round to zero. Reducing the substep to 0.005 s changes sampled pressure by at most 0.02456 Pa and local tissue volume by 2.89e-13 m³. These measurements concern this synthetic two-cell specimen only.


## Mixed RT0–P0 flow on arbitrary tetrahedra

`MixedDarcy` supplies an alternative to the orthogonal two-point interface. Flux has one globally oriented volume-flow degree of freedom per active face, while pressure is constant per tetrahedron. For the face opposite vertex `vi`, the normalized RT0 basis is `wi(x)=(x-vi)/(3*V)`, with unit integrated outward normal flux. The local resistance matrix is `Mij=integral mu*wi^T*K^(-1)*wj dV`. Its quadratic integrand is integrated exactly from the tetrahedral centroid and covariance. Internal faces share one unknown with opposite signs in their adjacent cells. The assembled positive-definite flux system solves `M*q=B^T*p` plus prescribed exterior face pressures. Unlisted exterior faces have zero normal flux. No centroid/normal alignment assumption is made.

Definitions are consistent with [FreeFEM's Raviart–Thomas space documentation](https://doc.freefem.org/documentation/finite-element.html#raviart-thomas-element) and the [DefElement tetrahedral RT0 reference](https://defelement.org/elements/examples/tetrahedron-raviart-thomas-legendre-0.html). The implementation is native Rust rather than a wrapper around either package. Scalar or full symmetric positive-definite spatial permeability tensors are supported, including off-diagonal coupling. Cholesky scaling handles small SI permeability values without an absolute determinant threshold. Invalid topology, overlapping neighbors, nonmanifold faces, non-SPD tensors, numerical overflow and inaccurate/nonpassive solves are rejected.

`DarcyResponse` reports owner-oriented face flow, cell-integrated outflow, reconstructed centroid Darcy velocity, pressure work, resistance dissipation and pressure-equation residual. Local divergence follows exactly from the signed sum of face fluxes divided by cell volume. For a closed domain, each internal face contributes equal and opposite volume rates. Pressure work and `q^T*M*q` agree to solver precision.

`CellPoreTissue::step_mixed_darcy` couples this flow to actual elastic FEM geometry and cell fluid inventories. Reference permeability is pushed forward as `Kcurrent=F*Kreference*F^T/J`; the current-geometry operator is rebuilt for each accepted transport evaluation. This is an objective referential conductivity law, not an empirically fitted strain-dependent pore-permeability model. Each internal face requires a matching unrestricted network edge. The adapter replaces only these pore-edge fluxes, retaining additional capillary and lymph exchange laws. Donor protein advection remains conservative. Actual accepted custom fluxes are retained by `LymphNetwork::rates`. Failed pressure, flux or FEM evaluations roll back both tissue and network.

Time integration is still explicit with user-controlled maximum step and positivity limits; it is not unconditionally stable or an implicit consolidation solve. The same temporal-refinement requirement applies. The standalone operator can accept prescribed exterior pressures; the tissue adapter uses sealed mesh boundaries and exchanges with other compartments through network edges. Gravity, species diffusion within tissue, cavitation and full vascular/lymph anatomy remain absent.

`cargo run --release -p physics --example mixed_fem_darcy` runs an oblique, anisotropic two-cell specimen with deformation, current pressure and conserved protein. Tests cover an exact linear-pressure/constant-flux patch with full permeability tensor, rotation objectivity, closed-face conservation, dissipation/work balance, permeability/viscosity scaling, manufactured quadratic-pressure refinement on 6/48/162 nonorthogonal tetrahedra (complete velocity-error integration, not centroid error alone), coupled local swelling, actual flux diagnostics, preservation of additional capillary exchange, invalid topology/tensors and transactional rollback.

The flux operator now stores local 4-by-4 tetrahedral resistance blocks and uses matrix-free diagonally scaled conjugate gradients. An implicit coupled pressure/solid solve, measured permeability/storage and actual anatomical volume meshes are still needed for full-organ simulations. This removes the orthogonality restriction for the supported mixed operator; it does not establish calibrated anatomical perfusion or completion of the whole-body goal.

Measured oblique mixed-flow example (10 s, maximum substep 0.01 s): both pore pressures approach 26.394329700 Pa. Final local tissue volumes are 2.003727215318e-7 and 1.168840868924e-7 m³. At the 100 saved samples the total-fluid and protein differences round to zero. Halving the substep to 0.005 s changes sampled pressure by at most 0.06749 Pa and tissue volume by 7.44e-13 m³. These measurements are for a synthetic specimen and do not supply human organ calibration.


## Matrix-free flow scaling

`MixedDarcy` no longer assembles or factorizes a dense global face matrix. It stores at most sixteen resistance entries per tetrahedron, gathers face vectors into each cell, applies the local matrix and scatters the result. Operator storage is linear in cells/faces/vertices. The only dense Cholesky inversions remaining are the 3-by-3 permeability tensors.

`DarcySolve` provides a bounded iteration count and relative scaled-residual tolerance; the default is 2000 iterations and 1e-13. The operator diagonal supplies Jacobi scaling. Right-hand-side normalization avoids squaring extremely small SI flow scales. The true residual is recomputed before acceptance and every 32 iterations; recomputation restarts the recurrence to control drift. Nonconvergence and nonpositive curvature return errors rather than approximate success. The independent pressure-residual and passivity checks remain active. `DarcyResponse::solver_iterations` and `MixedDarcy::operator_storage_bytes` expose actual iteration count and owned operator payload (excluding constructor scratch/vector headers).

Resource bounds now permit 250,000 cells and 1,000,000 active faces; these bounds are not a claim that every such ill-conditioned mesh converges under the default iteration limit. The fluid/protein network bounds likewise allow 250,000 spaces and 1,000,000 edges. `CellPoreTissue` builds a hash index for mapped face edges rather than scanning the entire edge list once per face. Duplicate pore-edge mappings remain errors.

`cargo run --release -p physics --example darcy_scale -- 16` constructs an anisotropic linear-pressure patch with 24,576 tetrahedra and 50,688 active faces. In the measured local release run, operator payload is 11,612,160 bytes, the solve takes 31 iterations, maximum centroid-velocity error is 3.53e-18 m/s and pressure residual 3.88e-13 Pa. Construction took 0.02005 s and the flux response 0.01149 s in that single run; these timings are not anatomical FEM-step performance or general real-time guarantees. The 3,072-cell patch uses 1,453,056 operator bytes and 32 iterations. A separate coupled test actually transports fluid/protein through 3,072 FEM-associated spaces and more than 4,096 internal-face edges, verifying inventory/pressure consistency and conservation beyond the old graph limits.

Earlier shared-package argument-count errors required a temporary focused Cargo harness; those errors were subsequently fixed in the worktree. The current anatomical importer and the selected biomechanical/transport regression suites have now passed directly through the workspace physics package. Anatomical integration and calibration remain unfinished.


## Actual anatomical volume meshes

`tools/export_anatomical_tetrahedra.py` now builds constrained volume meshes from selected closed Human Reference Atlas structures. Nine meshes are present in `assets/anatomy/hra-female/tetrahedra`: left/right ovaries, left/right crystalline lenses two papillary muscles, a liver envelope and two right-lung lobe envelopes. Source GLB hashes must match the atlas manifest. Exact-position welding is performed, and disconnected/nested surfaces are rejected because anatomical region/hole semantics have not yet been supplied. The original atlas world frame and metre coordinates are retained.

TetGen 0.8.3 generates linear tetrahedra with a constrained surface, radius/edge target 1.4 and minimum-dihedral target 10 degrees. Facet/vertex merging is disabled. An initial experiment with default facet merging changed enclosed volume by 2.17e-6 relative; the exporter rejected it. Disabling merging reduced volume differences to double-precision roundoff. No convex hull, smoothing, decimation or anatomical repair replaces the original surface. Output audits check strictly positive cell volumes above the FEM 1e-15 m³ threshold, interior-face orientation, outward boundary orientation, source-surface distance of boundary vertices, total enclosed volume and boundary area. Per-file JSON records generator versions/options, anatomy IDs, source/license/attribution and hashes.

`TetraMesh::from_bytes` loads the bounded VXTM/1 binary format directly into the Rust biomechanical mesh. It rejects malformed size/version, nonfinite coordinates, invalid indices, inverted/degenerate cells, nonmanifold interfaces, incorrect boundary orientation, duplicate or missing boundary faces. `into_body` requires an explicit constitutive law and pin set. It does not invent organ-specific material properties or anatomical attachments.

Mesh counts: left ovary 462 points/1718 tetrahedra; right ovary 433/1588; right lens 2759/12034; left lens 2734/11845. Maximum audited boundary-vertex distance from the original surface is below 6e-17 m; relative enclosed-volume discrepancy is at most 2.35e-16. These numerical tolerances do not describe the atlas's biological uncertainty or patient accuracy.

`cargo run --release -p physics --example anatomy_tet` loads the actual left-ovary mesh into FEM and a closed spatial Darcy operator. Passing a `.vxtet` path inspects another mesh. `--swell` applies a synthetic uniform fluid-storage increment, with a numerical clamp at the lower 5% of the mesh; it is not a physiological ovarian ligament attachment or fitted edema simulation.

Measured left-ovary swelling specimen: initial volume 1.000789660248e-6 m³, converged volume 1.001490343196e-6 m³, pore pressure 43.989593353 Pa, 537 nonlinear iterations and force residual 9.93e-8 N against a 1e-7 N tolerance. Coefficients are explicitly synthetic (matrix shear 5000 Pa, bulk 50000 Pa, Biot coefficient 0.8, total storage Vreference/100000 m³/Pa, added fluid 0.001 Vreference). The right-lens flow specimen uses 22515 internal faces and converges in 102 iterations with residual 8.86e-12 Pa; summed closed-domain outflow is below 7e-26 m³/s. It does not model optical refraction, capsule/fiber layers or aqueous circulation.

Remaining: tissue-specific measured constitutive/permeability calibration, internal material regions, real attachments and contact, registration inside the female character, other organ tetrahedralizations and the full coupled body simulation. Surface-atlas completeness limitations still apply.

The cardiac volume assets contain 4044 points/16860 tetrahedra (anterolateral muscle) and 2879/11890 (posteromedial muscle). Their material laws can be assigned transactionally with `Body::set_myocardium_batch`; fiber fields and coefficients remain caller-supplied. See the tetrahedra README for the synthetic contraction diagnostic and current cardiac domain limitations.

Composite atlas boundary patches now provide audited liver and right middle/lower lung volume envelopes. Direct FEM tests verify rest-state geometry/volume and stresses. Internal vascular/biliary/airway domains, lung gas spaces and physiological constitutive calibration remain unresolved. The upper right-lung lobe has source self-intersections and the left assembly has open edges; neither is accepted as a volume.

`cargo run --release -p physics --example anatomy_transport -- --fixed` couples each of the 21,696 anatomical right middle-lobe cells to its own conservative fluid/protein inventory, using the actual internal-face connectivity and current RT0 fluxes. A 1e-4 s closed redistribution step moved 7.259151894521e-13 m³ internally, with fluid drift -1.761828530289e-19 m³ and protein drift 2.168404344971e-18 kg. This fixed-skeleton isolation verifies transport/storage, not organ deformation. Without `--fixed`, the same example solves coupled deformation with a lower-5%-height numerical clamp; that coupled step passed with internal redistribution 1.112443548404e-12 m³, fluid drift -2.710505431214e-19 m³ and protein drift 6.722053469410e-18 kg. Storage energy after deformation was 8.209731340258e-7 J. The coupling requires force-converged FEM and updates flow on current geometry; this remains a quasistatic numerical specimen. Synthetic storage, permeability and initial 0–100 Pa pressure gradient are numerical inputs, not measured alveolar or interstitial physiology.

### Anatomical transport time-resolution finding

`anatomy_transport` now supports `--seconds=`, `--max-step=` and `--output=` (cell inventory, pore pressure, volume ratio), and reports actual deformation and total-energy change. The coupled baseline decreased total energy from 4.267169904775e-6 to 1.871677707876e-6 J. However, fixed-skeleton runs to 0.001 s with maximum steps 1e-4, 5e-5 and 2.5e-5 s did not establish pressure convergence: successive RMS differences were 25.08 and 27.88 Pa, with maximum differences exceeding 1300 Pa despite the initial 0–100 Pa field. Global inventory conservation and energy decrease therefore do not prove accurate local pressure transport on these small anatomical cells. The explicit fluid integrator currently limits outgoing fluid fraction, not the pore-storage diffusion stability spectrum; storage-aware stability control or an implicit spatial pressure solve is required before claiming reliable anatomical time integration. Do not treat the diagnostic default step as a physiological accuracy setting.

### Implicit spatial pore-storage correction

`MixedDarcy::implicit_storage_step` now solves fixed-mesh, sealed backward-Euler diffusion by eliminating pressure into the SPD flux system `(M + dt Bᵀ S⁻¹ B) q = Bᵀ p_old`. Local 4×4 storage contributions preserve matrix-free linear storage. The returned response reports physical Darcy dissipation and residual at the new pressures, excluding the augmented storage term. Invalid storage/time, boundary exposure, overflow and inaccurate fluxes are rejected. A two-cell analytical backward-Euler test covers tiny/large steps, conservation, pressure bounds and exact dissipation plus numerical-energy loss.

Use `anatomy_transport --fixed --implicit --seconds=0.001 --max-step=0.001`. The 21,696-cell lung solve converged in 270 PCG iterations, pressure range 0.4057–99.4905 Pa and stored-fluid drift -1.2441e-21 m³. Successive step halvings (0.001, 0.0005, 0.00025, 0.000125 s) yielded decreasing RMS pressure differences 0.00254866282, 0.00152256897, 0.000839720171 Pa. Unlike the earlier explicit steps, no thousand-Pa transient appeared in this specimen. Eleven mixed-flow tests passed before an unrelated concurrent astrophysics edit broke workspace compilation; refinement runs used the just-built matching example binary. This API currently addresses sealed fixed geometry and linear pore storage only. Implicit coupling to deforming FEM, conservative protein transport, capillary sources and the full physiological network remain to be integrated.

### Implicit conservative protein advection

`MixedDarcy::implicit_protein_step` solves backward-Euler donor-upwind transport using the new cell fluid volumes and the same oriented face fluxes as pressure diffusion. A sparse positive Gauss-Seidel solve enforces the cell equations and global mass residual, with no clipping or post-hoc redistribution. Invalid states, overflow and nonconvergence return errors without mutating inputs. Closed face topology is required; capillary exchange, diffusion/reflection and deforming-FEM coupling remain outside this API.

Analytical tests preserve uniform concentration when half the donor water volume transfers, conserve protein, retain nonnegative inventories for very large signed flux steps, and reject invalid states. All 12 mixed-flow tests pass. On 21,696 anatomical lung cells over 0.001 s with eight implicit steps, protein drift was 3.686287386451e-18 kg and maximum uniform concentration error 2.784705799286e-10 kg/m³. The CSV export now includes each cell protein inventory in implicit mode. Workspace compilation is restored in this validation run; the previous concurrent astrophysics compilation failure is no longer present.

### Implicit pressure-reservoir exchange

`implicit_storage_step` now supports the existing prescribed boundary-face pressures; unspecified external faces remain sealed. Boundary reservoir pressure enters the flux right-hand side, while storage contributes only to the adjacent cell. Returned dissipation is physical Darcy dissipation at the accepted pressures. Organ storage energy can increase under external work; a sealed-system energy-decrease criterion is inappropriate for a driven port. The new one-cell compliance test verifies backward-Euler pressure, exchanged fluid and the full boundary-work/storage/dissipation/temporal-loss identity for small and large steps. All 13 mixed-flow tests pass.

`cargo run --release -p physics --example anatomy_reservoir` opens one explicitly reported triangle of the anatomical middle-lobe envelope to a synthetic 100 Pa reservoir, starting cells at 20 Pa. For dt=0.001 s, 21,696 cells converged in 173 iterations, gained 3.642990493810e-14 m³ and received 3.642990493810e-12 J boundary work; energy-ledger discrepancy was 2.405710216562e-26 J. Port indices `[3043,3042,1848]` identify a numerical boundary triangle, not a vessel. Real vascular attachment, time-varying blood reservoir state, deforming tissue and boundary protein exchange remain outstanding. `implicit_protein_step` still requires sealed faces and must not be used to silently discard external protein influx.

### Boundary protein exchange

`implicit_protein_step_with_boundary` now accepts external reservoir concentrations per face. Inflow requires an explicit nonnegative concentration; outflow uses accepted tissue concentration. Missing inflow concentration and internal-face reservoir assignments are rejected. The original sealed method still rejects open faces. Sparse transport convergence and the external mass ledger use a relative 1e-14 inventory tolerance; the earlier 1e-12 tolerance obscured small net boundary transfers despite acceptable whole-inventory error.

Analytical tests cover influx, efflux, zero initial protein with reservoir supply and missing-source rejection; all 14 mixed-flow tests pass. The anatomical pressure-reservoir example now initializes tissue protein concentration at 10 kg/m³ and reservoir concentration at 20 kg/m³. On 21,696 cells, protein gain is 7.285980977420e-13 kg and boundary mass-ledger discrepancy is -1.019981347989e-21 kg. This is donor advection through a numerical port; membrane reflection, protein diffusion, physiological concentration calibration and dynamic vessel coupling remain outstanding.

### Accepted-state reversal of anatomical exchange

The anatomical reservoir example now continues from the accepted pressure and protein inventories, then changes external pressure from 100 Pa to 0 Pa on the same port. It verifies outward water/protein transport, no assumed inflow concentration for external efflux, actual tissue-donor concentration, fluid/protein boundary ledgers and the storage/dissipation/temporal-loss energy identity. On the 21,696-cell middle-lobe mesh, the second 0.001 s step removed 1.270453210326e-14 m³ fluid and 1.279011472867e-13 kg protein. Energy-ledger discrepancy was -1.265206674230e-25 J. This demonstrates flux reversal with accepted-state continuity; it is not a registered vessel port, vascular wall model or deforming-organ circulation.

### Implicit storage–solid fixed-point coupling

`Body::implicit_cell_pore_step` now alternates converged elastic FEM and backward-Euler RT0 storage/flux solves on the current geometry. Each iterate reconstructs the old fluid-inventory pressure at the current solid volume, solves the storage system, and relaxes the candidate inventory. Acceptance requires both the configured force residual and the maximum cell storage-pressure residual; rejected/nonconverged iterations leave the original body and pore stores unchanged. Referential permeability continues to transform with `F K Fᵀ/J`. This is an iterative quasistatic elastic coupling, not a monolithic Newton solve, inertial flow or viscoelastic history integration.

Tests verify actual deformation, force/storage convergence, sealed inventory conservation, energy decrease and rollback after pressure/geometry failures on an oblique two-cell specimen. All 34 mechanics/pore/flow tests pass. `anatomy_transport --coupled-implicit` additionally applies conservative protein advection to the accepted volumes and converged face flows; the anatomical middle-lobe run converged in 24 outer iterations, pressure residual 5.672393417200e-7 Pa and force residual 8.604918078081e-8 N. Fluid drift was -8.131516293641e-20 m³, protein drift 6.288372600416e-18 kg, final energy 1.871677731726e-6 J and maximum displacement 1.753341886075e-5 m. The base Body step commits geometry and fluid; the joint wrapper described below also commits its cell protein inventory. Integration with additional network compartments remains outstanding.

### Joint storage/solid/protein commit

`Body::implicit_cell_pore_protein_step` now stages the converged geometry/fluid update and subsequent protein advection in a private trial. No caller state changes until both phases succeed. The anatomical `--coupled-implicit` example uses this joint API. A regression verifies protein conservation and concentration recovery, then deliberately rejects an invalid protein inventory after successful FEM/storage convergence and checks unchanged geometry, fluid stores, energy/gradient and protein input. All three implicit-coupling tests pass; the example passes release compilation. This is a sealed elastic-organ transaction; network reservoir inventories, vessel wall exchange and viscoelastic history integration are not yet part of the joint step.

### Finite shared vascular-reservoir storage

`MixedDarcy::implicit_reservoir_step` now couples all listed external pressure ports to one finite linear-compliance compartment. Reservoir pressure changes by `dt * sum(boundary outflow) / C`; tissue and reservoir storage equations are solved together. A matrix-free rank-one port contribution accounts for cross-port coupling without allocating a dense face matrix. Returned dissipation remains the physical Darcy dissipation at accepted tissue/reservoir pressures. Inputs do not mutate; invalid compliance/pressure, absent ports, overflow and inaccurate solves are rejected.

A two-port analytical specimen verifies aggregate flow, total stored-fluid conservation and the joint tissue/reservoir dissipation plus temporal-loss identity for small and large steps. All 15 mixed-flow tests pass. The anatomical middle-lobe example uses synthetic reservoir compliance 1e-14 m³/Pa: pressure falls from 100 to 96.51567647469 Pa while tissue gains 3.484323525309e-14 m³. Combined stored-fluid drift is 9.655657479905e-28 m³ and joint energy discrepancy is 6.058451752097e-28 J. This advances finite-compartment coupling but does not supply an absolute reservoir inventory/reference volume, finite reservoir protein balance, anatomical vessel attachment or full blood circulation. Those caller states and their joint transaction still need integration.

### Finite vascular protein inventory

`implicit_protein_reservoir_step` treats all exposed ports as exchanges with one finite, mixed protein compartment. It solves tissue and reservoir donor-upwind inventories together using accepted new fluid volumes; reservoir concentration is computed from its evolving mass/volume rather than prescribed indefinitely. Closed combined-system mass is checked by the same sparse positive solve used for other protein boundaries. Invalid inventories, absent ports, overflow and nonconvergence return errors without mutating inputs.

Analytical tests cover simultaneous opposing ports, mixing at large steps, conservation and preservation of uniform concentration when reservoir water transfers to tissue. All 16 flow tests pass; the three implicit mechanics/protein transaction tests also pass after the shared solver refactor. The anatomical middle-lobe example gives the finite reservoir an explicit initial volume 1e-9 m³ and concentration 20 kg/m³. Tissue gains 6.968647045003e-13 kg protein, the reservoir loses 6.968647050623e-13 kg, and combined drift is -5.620441846292e-22 kg. These finite inventories still need a shared persistent transaction with deforming FEM and the blood/lymph networks; wall selectivity and physiological calibration are not supplied by donor advection.

### Persistent atomic finite-compartment exchange

`PoreReservoir` stores reference/current absolute fluid volume, reference pressure, linear compliance and protein mass, with checked pressure evaluation. `MixedDarcy::implicit_reservoir_transport_step` stages accepted tissue pressures/volumes/protein and reservoir state, validates positive finite fluid inventories and combined water/protein balances, then commits all caller slices and compartment together. A late protein failure leaves every input unchanged. This transaction uses fixed geometry; the deforming Body transaction remains a separate sealed API.

The new regression follows repeated exchanges, verifies decreasing pressure difference and absolute combined inventories, and deliberately triggers a post-pressure protein rejection to verify rollback. All 17 mixed-flow tests pass. The anatomical middle-lobe example now continues persistent state for three 0.001 s steps: reservoir pressures 96.51567647471, 93.51216399590 and 90.70878913832 Pa. Combined fluid drift is -1.626303258728e-19 m³ and protein drift 6.505213034913e-19 kg. Actual vascular port registration, pressure-volume laws fitted to vessel walls, reflection/diffusion and connection to blood/lymph circuits remain outstanding.

### Deforming tissue and finite-compartment transaction

`Body::implicit_cell_pore_reservoir_step` couples elastic equilibrium, local
backward-Euler fluid storage, a finite compliant reservoir and conservative
protein advection. Ports retain their boundary vertex identities as the tissue
deforms; permeability is pushed forward into the current geometry. Acceptance
requires both the FEM force tolerance and the maximum cell/reservoir pressure
residual tolerance. Protein uses the accepted fluid inventories and face flows.
Geometry, cell inventories, protein and reservoir commit together, including
rollback if protein validation fails after mechanics has converged.

Run `cargo run --release -p physics --example anatomy_finite_coupling` for the
21,696-cell anatomical middle-lobe envelope. The verified 0.0001 s step required
29 outer iterations: force residual 8.507086279925e-8 N, pressure residual
7.848450751524e-7 Pa, reservoir pressure 99.38670592117 Pa and maximum displacement
6.360926367216e-6 m. Combined fluid drift was -3.252606517457e-19 m³ and protein
drift -9.540979117872e-18 kg. The port `[3043,3042,1848]`, lower-five-percent clamp,
material, permeability, storage and initial pressures are numerical scenarios,
not fitted pulmonary tissue or a registered vessel. The solid is quasistatic and
elastic; viscoelastic histories, inertia, osmotic reflection/diffusion, nonlinear
vessel walls and airway/gas exchange are not included in this transaction.

The oblique two-cell regression also continues four accepted deforming steps,
checking joint fluid/protein conservation, nonnegative protein, decreasing
combined elastic/storage/reservoir energy and late-failure rollback. These are
numerical consistency checks, not physiological validation or an anatomical
time/mesh-convergence study.

### Repeated anatomical exchange and time-step sensitivity

`anatomy_finite_coupling` accepts `--steps=N`, `--seconds=DT` and
`--output=PATH.csv`. Every step starts from accepted geometry, cell fluid/protein
and reservoir state, and checks combined inventories against the initial totals.
CSV contains final cell pressure, absolute fluid volume and protein mass, with
17 significant decimal places. The exported pressure includes the actual local
solid volume change in the Biot storage equation.

To reproduce the deforming time-step comparison, run the following with
`N=1,2,4,8` and `DT=0.0001/N`:

```sh
cargo run --release -p physics --example anatomy_finite_coupling -- \
  assets/anatomy/hra-female/tetrahedra/left-ovary.vxtet \
  --steps=N --seconds=DT --output=/tmp/finite-N.csv
```

For the 1,718-cell mesh at the common final time 0.0001 s, unweighted cell-pressure
RMS differences from the eight-step result were 0.020885409879, 0.010361286191 and
0.004381627659 Pa for one, two and four steps. Maximum differences were
0.560152875313, 0.270819925794 and 0.097447859343 Pa. Final reservoir pressures
were 98.53544907524, 98.51616807898, 98.50654275004 and 98.50137319336 Pa.
All runs met 1e-7 N force and 1e-6 Pa coupled-pressure tolerances and retained
fluid/protein. These decreasing differences support temporal refinement in
this scenario; the eight-step run is not an analytical reference, and mechanical
solver tolerance and spatial discretization errors were not separated. This
uses the same synthetic constants as the lung example, not ovarian measurements.
The pinned middle-lobe two-step case also passes, with zero displacement and
final fluid/protein drifts 1.355252715607e-20 m³ / 1.084202172486e-18 kg.

### Signed exchange with the vascular circuit

`Circulation::step_with_exchange` accepts per-compartment signed integrated
fluid transfers in m³ (positive into blood, negative out), then solves implicit
vessel flow and updates pressure from the supplied end-of-step elastance and
external pressure. It stages on a cloned circuit and commits only after the
circulation solve succeeds. Depleted staged inventories, invalid transfers or a
failed solve preserve volumes, pressures and vessel flows. The reported volume
drift is relative to the inventory including the prescribed net exchange.

The two-compartment analytical regression verifies
`q = E*(V0+dV0-V1-dV1)/(R+2*dt*E)`, both individual volume balances, pressure-law
consistency, successive influx/outflux and rollback. This is the circuit-side
fluid exchange operation; it does not yet compute simultaneous capillary/FEM
exchange pressures or transport plasma protein through the vascular graph.
Calling this and the tissue step in sequence would be operator splitting and
must be checked for time-step error before being described as a coupled solve.

### Conservative protein circulation

`Circulation::step_with_protein` stages a closed-circuit blood-flow solve and
backward-Euler mixed-compartment protein transport together. Signed accepted
vessel flows select the donor; concentration uses accepted compartment volume.
The positive conservative transport matrix uses the same checked sparse solver
as tissue protein transport, without clipping or mass redistribution. A failed
flow solve, invalid protein state or unsuccessful transport preserves both
circuit and caller protein. Inventories are in kilograms and each compartment
is treated as homogeneous fluid; hematocrit/plasma partition is not resolved.

Regression tests compare direct and reverse one-vessel transport to its exact
discrete solution, preserve uniform concentration over repeated flow steps,
check late-error rollback, and follow 200 steps of a driven three-compartment
closed loop with heterogeneous initial concentration, an initially empty
protein compartment and reversing flow. Combined mass remains within 1e-11
relative tolerance without negative masses. All ten circulation tests pass;
the existing seventeen mixed-Darcy and four implicit-pore tests also pass.
External volume/protein exchange and pressure coupling to deforming tissue are
still separate operations; this API does not silently assign an external protein
concentration or advance the tissue. Protein binding, synthesis, degradation,
red-cell/plasma separation and osmotic pressure feedback remain outstanding.

`Circulation::step_with_protein_exchange` now stages signed external fluid and
protein inventories, implicit vascular flow and conservative vessel advection in
one transaction. Callers supply matching integrated m³/kg transfers from their
external exchange model; positive values enter blood. No external concentration
or protein reflection is invented. Volume diagnostics exclude the prescribed
net transfer, while protein transport conserves the staged total. Invalid or
depleting transfers and late transport errors preserve all accepted state.
The new twenty-step alternating influx/outflux regression preserves uniform
concentration when external transfers carry that concentration, and checks
separate accumulated fluid/protein ledgers. A finite-individual-mass but
overflowing-total case rejects after the staged flow solve and verifies rollback.
The pressure-dependent exchange itself remains outside this API; simultaneous
tissue/vascular pressure convergence and shared external inventory transactions
still need to be assembled and verified.

### Tissue response at prescribed vascular trial pressure

`Body::implicit_cell_pore_boundary_protein_step` solves deforming elastic tissue
and backward-Euler pore storage at explicitly prescribed boundary-triangle
pressures. Unlisted exterior faces remain sealed. Accepted face flows then
transport protein, using an explicitly supplied homogeneous exterior
concentration for inflow and accepted tissue concentration for outflow.
Mechanics, local fluid inventories and protein commit together. The exterior
fluid/protein ledger is caller-owned; prescribing pressure is not a closed
reservoir or a complete vascular coupling.

The new oblique-tetrahedron regression verifies actual deformation, both
convergence tolerances, retained fluid matching integrated boundary flow within
the storage-equation tolerance, protein gain matching boundary influx and a
late invalid-concentration failure preserving geometry, energy and protein.
This supplies a pressure-controlled tissue trial needed by a future simultaneous
circuit/tissue solve. No vascular attachments or fitted organ parameters are
introduced, and viscoelastic/inertial tissue remains unsupported by this solver.

### Simultaneous vascular/tissue pressure and protein coupling

`Body::implicit_vascular_pore_step` now iterates the pressure and homogeneous
protein concentration of one vascular compartment against a deforming porous
tissue body. All specified tissue ports share that compartment. Each iteration
restarts both systems from their old accepted inventories, solves the tissue at
trial pressure/concentration, applies signed port fluid/protein transfers
to blood, and solves implicit vessel flow and protein transport. Relaxed trial
values converge to accepted blood pressure and concentration. Acceptance requires
explicit pressure and concentration tolerances as well as both subsolvers'
tolerances and combined water/protein checks. No trial history/state is committed
until all checks pass; nonconvergence leaves both bodies and both protein slices
unchanged.

Regressions follow three consecutive deforming exchanges with joint conservation
and test complete rollback when the outer iteration budget is exhausted. A
separate all-pinned single-tetrahedron case compares the coupled tissue and two
vascular pressures to an independently assembled three-storage backward-Euler
linear system, within 3e-6 Pa, while retaining uniform protein concentration.
The unknowns include both vascular vessel flow and tissue storage, so accepted
pressure is not taken from a sequential standalone reservoir update.

This is quasistatic elastic tissue with homogeneous blood and one shared vascular
port compartment. It lacks reflection/diffusion/oncotic feedback, red-cell/plasma
partition, spatial vascular wall mechanics, multiple independently connected
port compartments and fitted anatomical vascular attachments. Fixed-point
convergence is not guaranteed for arbitrarily stiff exchange; exhaustion is an
explicit failure rather than an accepted approximate state. Anatomical validation
and time/mesh refinement of this combined solver remain required.

The anatomical driver exposes the simultaneous mode via `--vascular`:

```sh
cargo run --release -p physics --example anatomy_finite_coupling -- \
  assets/anatomy/hra-female/tetrahedra/left-ovary.vxtet \
  --vascular --steps=2 --output=/tmp/anatomy-vascular.csv
```

It connects the reported first exterior triangle to compartment zero of a
synthetic two-compartment circuit (initial pressures 100/20 Pa, each volume
1e-9 m³, compliance 1e-14 m³/Pa, vessel resistance 1e12 Pa s/m³). Initial protein
concentrations are 20/10 kg/m³ in blood and 10 kg/m³ in tissue. It reports outer
vascular pressure/concentration convergence and actual vessel flow alongside
the tissue mechanics/storage convergence, displacement and combined inventory
drifts. Every step retains accepted circuit, tissue and protein state. These
parameters and the boundary port remain numerical choices, not anatomical
vascular registration or measured ovarian properties.

`--trace` emits one-based outer iteration, pressure residual in Pa and
concentration residual in kg/m³; `--vascular-iterations=N` controls its limit.
`implicit_vascular_pore_step_with_observer` exposes these same diagnostics to
callers without changing staged states. Observer side effects are caller-owned.
The regression verifies one diagnostic callback on a deliberately exhausted
one-iteration solve and preservation of all physical state.

The anatomical deforming run with the original 1e-7 N solid / 1e-6 Pa tissue
tolerances failed after 200 vascular iterations. A traced 35-iteration repeat
showed pressure residuals oscillating around 1e-5 to 1e-3 Pa instead of meeting
the 1e-6 Pa vascular tolerance. The all-pinned comparison converged in 22 outer
iterations with 6.986928724473e-7 Pa pressure residual, 1.776356839400e-14 kg/m³
concentration residual and combined water/protein drifts -5.293955920339e-22 m³
/ 4.235164736272e-21 kg. Thus the deforming anatomical example cannot be claimed
converged using the original defaults. `--solid-tolerance=VALUE` and
`--tissue-pressure-tolerance=VALUE` allow stricter internal solves while retaining
the vascular acceptance criterion; convergence must be verified for that choice.

The deforming 1,718-cell anatomical case converged with solid tolerance 1e-10 N
and tissue pressure tolerance 1e-9 Pa: 22 vascular iterations, pressure residual
4.970376465963e-7 Pa, concentration residual 1.421085471520e-14 kg/m³,
35 final tissue iterations, solid residual 8.826517575698e-11 N and tissue
pressure residual 9.017400159905e-10 Pa. Port pressure was 97.77695501103 Pa,
vessel flow 7.700688614953e-11 m³/s and maximum displacement
1.230327142848e-6 m. Combined water/protein drift was -2.117582368136e-21 m³ /
-8.470329472543e-22 kg. The final CSV has 1,718 finite records with positive fluid
and nonnegative protein. These stricter internal tolerances are now defaults
for `--vascular`; standalone reservoir mode retains its earlier defaults.
The outer acceptance tolerances were not relaxed. This is one anatomical step,
not yet a time-refinement or tissue-parameter validation of the combined solver.

Vascular CSV export now also writes `PATH.csv.blood.csv` with accepted
compartment pressure, fluid volume and protein mass. It is generated after all
requested steps succeed. `tools/compare_pore_states.py REFERENCE.csv COARSER.csv`
checks matching IDs, finite states, positive fluid and nonnegative protein,
then reports unweighted pressure/concentration RMS and maximum differences.
When both vascular companions exist, it also compares blood states and combined
inventory differences. A companion present for only one input is rejected,
so tissue-only inventory cannot masquerade as a joint balance. The reference
file is a numerical comparison, not automatically an exact solution.

For temporal refinement of the combined anatomical solver, use `--vascular`
with `--steps=2 --seconds=0.00005` and `--steps=4 --seconds=0.000025`, yielding
the same final time 0.0001 s. Each run must first pass its convergence and
joint-inventory checks before exported states can support a refinement claim.

The deforming 1,718-cell two-/four-step runs both completed to 0.0001 s, each
accepted step meeting the existing vascular pressure/concentration, tissue
pressure and force tolerances. Relative to the four-step result, tissue-pressure
RMS differences were 0.017408159001 Pa for the earlier one-step result and
0.006457127965 Pa for two steps; maxima were 0.465673413521 / 0.175832130015 Pa.
Protein-concentration RMS differences were 8.316585565461e-7 / 2.855459743662e-7
kg/m³. The two-/four-step vascular pressure difference was 0.010880289788 Pa RMS
(0.014949469506 Pa maximum). Both combined exported inventories were
5.025949880559287e-7 m³ fluid and 5.035949880559287e-6 kg protein, with zero
inter-run difference at exported precision. Final port pressures were
97.74821523055 / 97.73326576105 Pa and maximum displacements
1.230362113032e-6 / 1.230385238812e-6 m. Final per-run fluid drifts were
-7.411538288475e-22 / -1.270549420881e-21 m³, protein drift -7.623296525289e-21 kg
in both. Decreasing differences support temporal refinement for this synthetic
coupled scenario; four steps do not establish an exact reference or a convergence
order, and spatial/mechanical solver errors and tissue calibration remain separate.

### Separate protein concentration on each tissue port

`implicit_cell_pore_boundary_protein_step_with_concentrations` assigns one
explicit exterior concentration to each supplied pressure boundary, in input
order. Canonical triangle vertex IDs map it into the assembled flow faces;
reordering triangle vertices does not change the assignment. Duplicate ports,
wrong-sized concentration arrays and nonfinite/negative concentrations fail
without committing tissue or protein. The homogeneous-concentration API now
delegates to this implementation. Each inflow uses its own port concentration;
every outflow uses accepted local tissue concentration.

The two-cell deforming regression exchanges through two independently driven
ports at +150/-150 Pa with exterior concentrations 20/5 kg/m³, then reverses the
drives on a fresh specimen. Both cases require simultaneous inflow and outflow
and check total protein change against the signed port ledger. Negative pressure
is a synthetic numerical suction case, not a desaturation/cavitation model.
Independent vascular-compartment coupling of these ports remains to be added;
the existing simultaneous vascular solver still attaches all ports to one
compartment.

### Independently connected vascular ports

`VascularPorePort` maps an explicit boundary triangle to its vascular compartment.
`implicit_vascular_pore_ports_step_with_observer` iterates pressure/concentration
for every connected compartment, passing each port's values into the deforming
tissue solve. Signed accepted face flows credit that exact compartment with
integrated fluid and donor protein; outflow concentration is accepted local
tissue concentration, inflow is the trial vascular donor concentration. Both
residuals are maxima over the connected compartments. The earlier single-
compartment methods delegate by assigning all their ports to the selected node.
Duplicate boundary triangles, including different vertex permutations assigned
to different compartments, are rejected. Full joint rollback is retained.

The new two-cell deforming regression connects its two exterior ports to two
different compartments with heterogeneous protein concentration, and follows
three accepted steps with simultaneous inflow/outflow. It verifies each vascular
node's water/protein ledger against its own pore transfer plus vessel transport,
both coupling tolerances, combined conservation and duplicate-port rollback.
The negative external pressure is a numerical suction scenario, not a fitted
venous pressure or cavitation model. All nine implicit-pore, eleven circulation
and seventeen mixed-Darcy tests pass. Independent anatomical port registration,
reflection/oncotic feedback and experimental calibration remain outstanding;
the earlier anatomical runs used the single-compartment implementation and
have not yet been repeated with independently connected ports.

The anatomical driver now accepts `--vascular-pair`. It attaches the first
exterior triangle to compartment zero and the triangle with the most distant
reference centroid to compartment one. It prints both node triples and signed
fluid/protein transfer into blood per port. This is a deterministic numerical
two-port scenario, not an anatomical arterial/venous registration. Pair mode
uses initial blood pressures 100/0 Pa, concentrations 20/10 kg/m³ and the same
strict vascular-mode tolerances and synthetic material/circuit constants.
For the left-ovary mesh the mapped triangles are `[63,62,67]` and `[125,124,5]`.
Use `--fixed` for the pinned comparison; omit it for deforming tissue. Successful
runs export final tissue and both vascular compartment states. The reported
boundary fluxes determine whether a port is actually inflow or outflow; geometry
or compartment names alone do not establish its flow direction.

The pinned 1,718-cell left-ovary two-port case converged at 0.0001 s in 22
vascular iterations: pressure residual 7.656667690981e-7 Pa and concentration
residual 4.645173135032e-11 kg/m³. First-port tissue outflow was
-1.238635602474e-10 m³/s (inflow), second-port outflow 6.210483282216e-12 m³/s.
Corresponding protein transfer into blood was -2.477271204949e-9 /
6.210483282216e-11 kg/s. Actual vessel flow was 9.676397996540e-11 m³/s; accepted
vascular pressures were 97.79372459788 / 1.029744632479 Pa. Fluid/protein drifts
were -6.352747104407e-22 m³ / 7.623296525289e-21 kg. Both companion exports were
checked for 1,718 tissue cells, both blood compartments, finite values, positive
fluid and nonnegative protein. This establishes the numerical pinned case;
deforming anatomy requires its own convergence evidence.

The corresponding deforming case completed in 23 vascular iterations with
pressure residual 2.671147854016e-7 Pa, concentration residual
2.332534165816e-11 kg/m³, solid residual 8.185682946014e-11 N and tissue-pressure
residual 8.765237424768e-10 Pa. Maximum displacement was 1.230305599542e-6 m.
The first/second port tissue outflows were -1.449832845207e-10 /
4.238427681685e-12 m³/s and protein transfers into blood -2.899665690415e-9 /
4.238427681685e-11 kg/s; actual vascular flow was 9.657625772350e-11 m³/s.
Fluid/protein drifts were -1.058791184068e-21 m³ / -5.929230630780e-21 kg.
Both final exports also passed finite-state/positive-inventory checks. This
confirms two independently connected numerical ports on deforming anatomy for
one step. These ports and synthetic constants are not fitted vascular anatomy;
multi-port temporal/spatial refinement and physiological validation remain open.

### Passive hydraulic interface resistance

`MixedDarcy::with_added_boundary_resistances` adds finite resistance on explicitly
active exterior pressure ports. Values are in Pa s/m³: the interface contributes
`R*q` pressure loss and `R*q²` power loss. Its contribution is added to the local
physical flux matrix and Jacobi scaling, so instantaneous flow, implicit storage
and finite-compliance reservoir solves all retain it. Reported dissipation now
includes that interface loss for operators constructed this way. Repeated calls
add further series resistance. The original operator is preserved on invalid
assignments, duplicate/reordered ports, negative/nonfinite values or overflow.

The new one-cell test compares signed flow to the exact series conductance
`G = 1/(1/G_tissue + R_interface)`, verifies power, implicit constant-pressure
storage and finite-reservoir exchange for zero, moderate and high resistance.
All eighteen mixed-Darcy and nine implicit-pore tests pass. This is a passive
hydraulic interface, without protein reflection, osmotic pressure or membrane
diffusion. The earlier anatomical results used pressure-controlled ports without
this additional resistance. Organ-specific wall parameters require measured data.

### Hydraulic interfaces in deforming vascular coupling

`implicit_cell_pore_boundary_protein_step_with_resistances` retains explicit
per-port series resistance in every current-geometry Darcy/storage iteration
while solving elastic equilibrium and donor protein transport.
`implicit_vascular_pore_ports_step_with_resistances_and_observer` uses this tissue
response inside the simultaneous vascular pressure/concentration solve, including
independent compartment mappings and signed local fluid/protein ledgers. Previous
methods delegate with an empty resistance list; source concentrations and donor
rules are unchanged. Invalid resistance assignment or any unsuccessful solve
preserves tissue, circulation and both protein arrays.

The new deforming two-cell/two-compartment comparison starts both cases from the
same old state, with zero resistance versus 1e15 Pa s/m³ at both ports. In this
synthetic case both accepted pore flows fall below one percent of the zero-
resistance values. Both solves converge, preserve joint fluid/protein inventories
and reject a negative-resistance call without state changes. All ten implicit-
pore, eighteen mixed-Darcy and eleven circulation tests pass. This verifies the
numerical coupling; no measured endothelial permeability, protein reflection,
oncotic pressure, membrane diffusion or new anatomical wall model is implied.
Anatomical runs with nonzero interface resistance remain to be checked.

The anatomical driver now accepts `--interface-resistance=R` in vascular mode,
applying that finite nonnegative Pa s/m³ value to each selected port throughout
the coupled solve. Nonvascular use with nonzero resistance is rejected. For the
two-port ovarian scenario, `R=1e13` in the pinned case converged in 21 vascular
iterations with pressure residual 9.874749764549e-7 Pa and concentration residual
9.344525153665e-11 kg/m³. First-/second-port tissue outflows were
-7.428091319759e-12 / 1.455884366573e-12 m³/s, compared with
-1.238635602474e-10 / 6.210483282216e-12 m³/s without interface resistance.
Combined fluid/protein drift was 2.117582368136e-22 m³ / 1.948175778685e-20 kg.
This checked the pinned anatomical resistance case; deformation requires a
separate solve. The chosen resistance is synthetic, not fitted endothelium.

The deforming 1,718-cell two-port ovarian case with the same `R=1e13` converged
in 22 vascular iterations: pressure residual 4.906763848567e-7 Pa, concentration
residual 4.671996123307e-11 kg/m³, force residual 9.956080571960e-11 N and tissue
pressure residual 7.410339009084e-10 Pa. First-/second-port tissue outflows were
-8.496018712563e-12 / 9.927628278622e-13 m³/s (about 17.06 / 4.269 times smaller
than the earlier deforming zero-resistance case). Maximum displacement was
1.229836686923e-6 m; fluid/protein drift -3.176373552204e-22 m³ /
-1.270549420881e-20 kg. Final tissue/blood CSVs passed finite-state, positive-fluid
and nonnegative-protein checks. This run and 28 focused tests used an isolated
manifest importing the actual repository sources, because the workspace's
concurrently added `voxy_rush` package initially had no target. These results
verify the biomechanical paths, not a full workspace build or fitted capillary
physiology. Temporal/spatial refinement of nonzero-resistance anatomy remains open.

The temporary workspace-manifest error subsequently cleared; the same 28 focused
tests were repeated successfully through the regular workspace Cargo invocation.
The anatomical result above still came from the source-importing isolated driver.

### Selective exterior protein membranes

`ProteinMembrane` specifies exterior concentration (kg/m³), reflection fraction
sigma in [0,1] and diffusive conductance D (m³/s, permeability times area).
`MixedDarcy::implicit_protein_step_with_membranes` solves positive backward-Euler
transport with exterior flux
`J = (1-sigma)*q*C_donor + D*(C_tissue-C_exterior)` in kg/s, outward positive.
Inflow uses prescribed exterior concentration and outflow accepted tissue
concentration. Diffusion adds both a positive exterior source and accepted
tissue sink, retaining the signed external mass ledger. The earlier boundary
concentration API delegates with sigma=0 and D=0.

Convection/diffusion and reflection are the decomposition used in membrane
transport equations discussed by [Elliott et al., 2009](https://pmc.ncbi.nlm.nih.gov/articles/PMC2711286/).
Here the upwind donor concentration is an explicit numerical approximation;
it is not a fitted membrane pore-average concentration model, a nondilute
thermodynamic law or complete Kedem–Katchalsky coupling. Osmotic feedback on
fluid pressure is still absent. Exterior inventories remain caller-owned.

The new one-cell regression checks sigma=0/0.8/1, inflow/zero-flow/outflow and
zero/moderate/large diffusion against the exact discrete mass equation and
external ledger. Full reflection with zero diffusion retains protein despite
fluid transfer; diffusion can populate an initially protein-empty compartment
at zero fluid flow. Invalid coefficients are rejected and accepted mass remains
finite/nonnegative. All nineteen mixed-Darcy, ten implicit-pore and eleven
circulation tests pass. Calibration of anatomical endothelial barriers remains
outstanding; the deforming transaction extension is described below.

### Selective protein exchange in the simultaneous vascular solve

`implicit_cell_pore_boundary_protein_step_with_membranes` accepts per-port
`ProteinMembrane` values alongside hydraulic resistances and stages mechanics,
fluid storage and selective protein transport together.
`implicit_vascular_pore_ports_step_with_membranes_and_observer` supplies trial
vascular concentrations and explicit `(triangle,sigma,D)` coefficients. Its
vascular source ledger uses the same accepted tissue concentration, trial blood
concentration, signed fluid flow, reflected donor flux and bidirectional
diffusion as the tissue solve. Thus pure diffusion is credited to the correct
vascular compartment even when fluid flow vanishes. Pressure/concentration
convergence and joint inventory checks precede the common commit. Older APIs
delegate with sigma=0, D=0; invalid/duplicate/unmapped membrane coefficients
leave all accepted tissue/circulation/protein state unchanged.

The new deforming two-cell/two-compartment regression follows three steps for
full reflection with zero diffusion and with D=1e-12 m³/s. The first retains
total tissue protein despite fluid exchange; the second transfers protein by
diffusion while preserving combined tissue/blood mass. Both preserve fluid,
converge and keep protein finite/nonnegative; an invalid reflection coefficient
verifies full rollback. All eleven implicit-pore, nineteen mixed-Darcy and
eleven circulation tests pass. The donor closure is still the stated numerical
approximation. Osmotic feedback, fitted membrane coefficients and anatomical
validation of selective exchange remain outstanding.

The anatomical example exposes `--protein-reflection=SIGMA` and
`--protein-diffusion=D` in vascular mode, independently of
`--interface-resistance=R`. SIGMA must be finite in [0,1]; D must be finite and
nonnegative in m³/s per selected port. Zero defaults reproduce unrestricted
donor advection. Port diagnostics now use the same reflected-advection plus
diffusion law as the joint solver, instead of reporting unfiltered donor flux.
These flags assign uniform coefficients across the selected ports; the library
API supports explicit individual assignments. Osmotic effects on fluid flow
are not introduced by these flags.

The pinned 1,718-cell ovarian case with R=1e13 Pa s/m³, sigma=1 and D=0
converged in 21 vascular iterations at 0.0001 s. Both accepted port protein
fluxes were zero while fluid outflows remained -7.428091319759e-12 /
1.455884366573e-12 m³/s. Initial tissue protein independently reconstructed
from the tetrahedral reference volumes was 5.005949880559287e-6 kg; accepted
CSV tissue protein was 5.0059498805593055e-6 kg (difference
1.8634724839594607e-20 kg). Combined fluid/protein drifts were
2.117582368136e-22 m³ / 1.948175778685e-20 kg. This verifies full retention for
the numerical anatomical barrier despite water transfer; it does not identify
human membrane coefficients or validate osmotic pressure feedback.

The deforming case at R=1e13, sigma=0.8, D=1e-15 m³/s per port converged in
22 vascular iterations: pressure residual 4.906763848567e-7 Pa, concentration
residual 4.616929061285e-11 kg/m³, force residual 9.956080571960e-11 N and tissue
pressure residual 7.410339009084e-10 Pa. Port protein transfers into blood were
-3.399409801374e-11 / 1.985545584089e-12 kg/s; fluid flows remained
-8.496018712563e-12 / 9.927628278622e-13 m³/s, matching the unrestricted-protein
hydraulic scenario because osmotic feedback is absent. Maximum displacement
was 1.229836686923e-6 m. Combined fluid/protein drifts were
-3.176373552204e-22 m³ / -1.270549420881e-20 kg. Final exports passed finite-state,
positive-fluid/nonnegative-protein checks, and tissue protein gain matched the
integrated signed port protein ledger within the mass-equation tolerance.
This is one anatomical numerical step with synthetic coefficients, not measured
endothelial selectivity or temporal/spatial convergence of the selective solver.

## Osmotic feedback in simultaneous vascular pore exchange

`implicit_vascular_pore_ports_step_with_osmosis_and_observer` adds the explicit phenomenological law `Pi = slope*C`, where C is protein mass concentration (kg/m3) and slope is supplied in Pa m3/kg. The tissue boundary uses `p_b - sigma*slope*(C_b-C_t)`. Both blood and local tissue concentrations iterate at accepted fluid volumes, alongside vascular pressures and deforming tissue storage. Acceptance checks the effective boundary pressure as well as blood pressure and both concentrations. Zero slope retains the previous API behavior.

The passive hydrostatic-minus-osmotic pressure convention follows [Pinsky, Fluid and Osmotic Pressure Balance and Volume Stabilization in Cells (2021)](https://doi.org/10.32604/cmes.2021.017740). This implementation does not reproduce that paper's ionic transport, membrane potential or pump-leak model. A linear protein mass-concentration slope is a user-supplied approximation; no physiological coefficient is inferred. Plasma oncotic nonlinearity, multiple solutes, Donnan effects and active regulation remain absent. No thermodynamic energy-decrease claim is made for the joint solute solver.

The anatomical driver accepts `--osmotic-slope=VALUE` in vascular mode (default zero). Example: `cargo run --release -p physics --example anatomy_finite_coupling -- assets/anatomy/hra-female/tetrahedra/left-ovary.vxtet --vascular-pair --interface-resistance=1e13 --protein-reflection=0.8 --protein-diffusion=1e-15 --osmotic-slope=8`. These coefficients and attachment ports are numerical demonstration inputs, not measured ovarian vascular anatomy.

The fixed-tetrahedron regression tests hydro-osmotic cancellation, both directions of protein-gradient-driven water exchange, the Darcy law recomputed at accepted concentrations, combined water/protein conservation and atomic rejection of invalid slope or exhausted iteration. The equilibrium case also exposed an overly strict relative flux residual near zero driving pressure; verification now includes a 64-epsilon pressure-roundoff floor while retaining its relative residual criterion. Focused release verification: 12 implicit-pore, 19 mixed-Darcy and 11 circulation tests passed.

Deforming left-ovary numerical run above passed: 1718 cells, 21 vascular iterations, effective-pressure residual 9.408745711426e-7 Pa, concentration residual 9.376677212458e-11 kg/m3, force residual 8.641549631320e-11 N, tissue-pressure residual 8.296137110619e-10 Pa. First-port tissue outflow was -2.471049129217e-12 m3/s (previous zero-slope selective run: -8.496018712563e-12); second port +9.927151885470e-13 m3/s. Joint water drift -9.529120656611e-22 m3, protein drift -1.016439536705e-20 kg; maximum displacement 1.229813956624e-6 m. Exported 1718 tissue and two vascular records were independently checked finite, with positive fluid volume and nonnegative protein. This demonstrates coupled numerics on imported anatomy, not physiological calibration or a complete ovarian circulation.

### Independent nonlinear step reference for osmotic coupling

The osmotic regression now covers dt = 0.001, 0.01 and 0.1 s for each of the three pressure/protein-gradient cases (nine cases total). Besides evaluating the accepted Darcy law, it independently solves the backward-Euler water transfer using a scalar bisection reference. For transferred water x and vessel transfer y, the two equal-compliance vascular equations eliminate to `y = a*x/(1+2*a)`, `a = dt/(R*c)`. Accepted vascular volumes are `V0+x-y` and `V1+y`; vessel protein follows the appropriate implicit upwind donor equation for each sign of y. Tissue protein remains fixed for full reflection and zero membrane diffusion. The reference then solves `x = dt*g*(p_t-p_b+8*(C_b-C_t))`, with `p_t = 20-x/S`, updated vascular pressure and concentrations. It does not call the circulation or coupled solver for its root or protein transport; the mesh hydraulic conductance g is obtained once from a unit-pressure Darcy response.

The test checks water transfer, both vascular volumes, tissue and vascular pressure, and blood concentration against this root using bounds derived from the configured pressure/concentration tolerances. All nine cases passed in release mode. This is a fixed-geometry, single-solute numerical reference, not validation against living tissue measurements or proof of convergence for every anatomical mesh.

## Nonlinear protein osmotic pressure law

`OsmoticPressureLaw` supplies `Pi(C)=a*C+b*C²+c*C³`, with C in kg/m3 and coefficient units Pa m3/kg, Pa m6/kg2 and Pa m9/kg3. All coefficients must be finite and nonnegative, giving a monotone pressure law on nonnegative concentrations. Evaluation rejects overflow instead of clipping. `implicit_vascular_pore_ports_step_with_osmotic_law_and_observer` uses `p_b-sigma*(Pi(C_b)-Pi(C_t))` at each iteration and in its accepted-pressure residual. The previous linear API remains a wrapper with b=c=0. The law is identical on both sides; separate protein fractions and compartment-specific material fits are not implemented.

Polynomial colloid-pressure approximations and their dependence on protein composition are discussed in [Nitta et al., The Corrected Protein Equation to Estimate Plasma Colloid Osmotic Pressure and Its Development on a Nomogram (1981)](https://www.jstage.jst.go.jp/article/tjem1920/135/1/135_1_43/_article/-char/en). This API does not embed their fitted coefficients or claim validity for a particular plasma, interstitial fluid, temperature or concentration range. Positive-coefficient polynomial extrapolation is only a numerical constitutive law; experimental calibration and bounded validity ranges remain necessary.

The anatomical driver adds `--osmotic-quadratic` and `--osmotic-cubic` (both default zero), retaining `--osmotic-slope` for a. The fixed-cell independent bisection reference now exercises both linear (8,0,0) and nonlinear (8,0.1,0.001) numerical coefficients, three hydro-osmotic conditions and three time steps: 18 cases. Reference polynomial evaluation is independent of the production evaluator. Tests also verify atomic rollback for negative or NaN coefficients and concentration-dependent pressure overflow. Release checks passed: 13 implicit-pore, 19 mixed-Darcy, 11 circulation tests.

The deforming left-ovary run with a=8, b=0.1, c=0.001, reflection 0.8, interface resistance 1e13 Pa s/m3 and diffusion 1e-15 m3/s passed at dt=1e-4 s: 1718 cells, 21 vascular iterations, effective-pressure residual 9.361656252427e-7 Pa, concentration residual 9.344347517981e-11 kg/m3. Tissue force residual was 6.467147468515e-11 N and pressure residual 6.062856883204e-10 Pa. First-port outward flow +3.154801300018e-13 m3/s reversed the linear-law run's inflow; second-port outward flow +9.926938156902e-13 m3/s. Joint water drift -1.058791184068e-21 m3 and protein drift -6.776263578034e-21 kg. Exported 1718 tissue records and two blood compartments were independently checked finite, with positive fluid volume and nonnegative protein. This illustrates sensitivity to the constitutive pressure law with demonstration coefficients; it does not establish ovarian physiology.

## Space-specific nonlinear osmosis in tissue/lymph exchange

`LymphNetwork::rates_with_osmotic_laws` and `step_with_osmotic_laws` accept one explicit `OsmoticPressureLaw` per fluid space. This permits different pressure/concentration relations for plasma, interstitial fluid and lymph. Hydraulic drive is `p_from-p_to+pump-sigma*(Pi_from-Pi_to)`; donor advection, reflection, diffusion and valve gating retain the existing conservative exchange law. The network integrates with positivity-limited explicit substeps; this is not an implicit solver or an error-controlled integrator. Accepted pressures/rates and inventories remain atomic on rejection.

`CellPoreTissue::step_mixed_darcy_with_osmotic_laws` connects these laws to the existing current-geometry RT0/FEM tissue coupling. Each trial fluid state re-equilibrates the solid and updates its actual Biot pressures. Internal pore-face exchanges retain RT0 advection; additional capillary and lymph edges use the space-specific nonlinear osmotic laws. The original `step_mixed_darcy` delegates with no override. This extends the existing distributed-tissue/lumped-network adapter; it does not merge the separate implicit circulation solver into a full cardiovascular/lymphatic monolithic solve.

Verification includes a nonlinear closed three-space valved network and a four-space loop with two deforming tissue cells, a capillary space, lymph inlet and prescribed pump return. Accepted capillary flux and protein advection are independently evaluated from the polynomial and actual pressures/concentrations. Combined fluid and protein inventories are conserved, cell/network volumes match exactly, and malformed law arrays or FEM exhaustion preserve accepted geometry and network inventories/diagnostics. Release verification passed: lymph 7, mixed Darcy 20, cell poroelasticity 5, implicit pore 13, circulation 11 tests. Numerical nodes/edges and coefficients are synthetic, not anatomically registered lymph vessels. Active lymphangion wall mechanics, actual valve geometry, lymphatic attachment fields and experimentally calibrated protein composition remain absent.

## Volume-dependent active lymphatic walls

`LymphaticWallLaw` represents one fixed-length cylindrical fluid space with actual radius `r=sqrt(V/(pi*L))`, stretch `s=sqrt(V/Vref)`, and pressure `P=Pext+scale*(exp(k*(s-1))-s^-3)+Tactive/r`. The passive pressure is zero at reference volume, rises with inflation and becomes negative under collapse. The active term follows thin-wall Laplace balance. The qualitative exponential stiffening/inverse-power collapse form and active tension/radius construction are motivated by [Bertram et al., Simulation of a Chain of Collapsible Contracting Lymphangions With Progressive Valve Closure](https://pmc.ncbi.nlm.nih.gov/articles/PMC3356777/). This is an explicit simplified constitutive law, not a reproduction of that paper's fitted model or measured coefficients.

`LymphNetwork::step_with_wall_and_osmotic_laws` evaluates these wall pressures at every trial volume and accepted state, with optional walls per node, nonlinear space-specific protein osmosis and existing valves. Activation tension and external tissue pressure are supplied for each call and held over that call; calcium dynamics, stretch-dependent activation, contraction timing/refraction and muscle length-tension regulation are not provided. Parameters are checked; collapsed-radius singularity, exponential/active-pressure overflow or failed integration rejects the entire state without clipping. Fixed length and circular cross-section limit the collapsed-vessel interpretation; wall inertia, viscoelasticity and resolved valve mechanics are absent.

`CellPoreTissue::step_mixed_darcy_with_lymphatic_walls` applies the same volume-dependent pressures to external network spaces while retaining actual equilibrated FEM/Biot pressures for tissue cells. A wall override for a tissue cell is rejected. This couples active lymphatic pressure with deforming tissue transport; it does not yet apply actual lymphatic surface tractions to an anatomically resolved vessel wall or automatically source external pressure from vessel/tissue contact.

Verification: a closed three-space network with no edge pump head completed five filling/contraction cycles. Filling opens the inlet and closes the outlet; active tension closes the inlet, opens the outlet and reduces chamber volume. Combined water/protein and initially uniform concentration remain conserved. Accepted active pressure agrees with the actual-volume wall equation; invalid/overflowing activation rolls back state and diagnostics. The deforming RT0 tissue test independently recomputes its external lymph-space pressure from the accepted volume. Release tests passed: lymph 8 and mixed Darcy 20; the previously present numerical demonstration coefficients remain uncalibrated.

### Initial first-order active lymphatic wall temporal convergence

A separate regression increases hydraulic conductance to 1e-9 m3/(Pa s) and alternates 0 and 0.03 N/m active tension over ten 0.1-second phases (five cycles), so wall-radius changes materially affect pumping. Its independent reference implements the three compartment water balances, ideal valve clipping, passive/active wall equation and integrated outlet volume directly, using classical RK4. No production pressure-law, flow-law or network integration calls are used in the reference. Halving the reference step from 1e-5 to 5e-6 seconds changed each final state component by less than 1e-16 m3.

| Explicit maximum step (s) | Maximum error across three final volumes and accumulated outlet volume (m3) | Accumulated outlet volume (m3) |
| --- | --- | --- |
| 0.01 | 4.700307138675e-10 | 1.664481526002e-8 |
| 0.005 | 2.302132631900e-10 | 1.640499780934e-8 |
| 0.0025 | 1.139369441435e-10 | 1.628872149029e-8 |

RK4 reference accumulated outlet volume: 1.617478454615e-8 m3. Errors reduce approximately by half on each step halving, consistent with first-order explicit integration. The regression also checks conservation of total volume and preservation of initially uniform protein concentration. Conservation alone is insufficient to establish dynamic accuracy: the coarsest step overestimates cumulative output by approximately 2.9 percent in this numerical fixture. This is time-discretization validation of the supplied lumped law, not physiological calibration, spatial convergence or validation of resolved lymphatic anatomy.

Reproduce: `cargo test --release -p physics --test lymph active_wall_cycle_refines -- --nocapture`.

### Conservative second-order active-wall integration

Active-wall network calls now use SSPRK2 (Heun): pressure/flux is evaluated at the old state and a positive Euler predictor, and accepted edge transfers are the trapezoidal mean of these two rates. Water and protein use identical opposite edge transfers at each stage; neither inventory is clipped or redistributed. Outgoing-volume/protein limits apply at both stages. If the predictor's outgoing limit is violated, the step is halved and retried (at most 64 trials); invalid constitutive states still fail atomically. The wall-enabled RT0/FEM adapter re-equilibrates the tissue at both stage volumes and at its final accepted state. The existing generic pressure/flux callback API and adapters without walls retain their original explicit integration. This remains explicit and lacks local truncation-error control.

The same independent RK4 fixture now gives:

| Maximum step (s) | Maximum final-state error (m3) | Accumulated outlet volume (m3) |
| --- | --- | --- |
| 0.01 | 1.604791445912e-11 | 1.615873663169e-8 |
| 0.005 | 3.890360434402e-12 | 1.617089418571e-8 |
| 0.0025 | 9.576207863350e-13 | 1.617382692536e-8 |

Errors decrease approximately fourfold on step halving. At 10 ms, outlet-volume relative error is approximately 0.0992 percent, versus 2.91 percent for the initial Euler result (about 29.3-fold reduction). These measurements apply only to this explicit numerical fixture. The regression now enforces second-order refinement, exercises predictor outgoing-limit retries without clipping, and checks rollback when an initially finite wall law overflows at the second-stage volume. Full focused release verification passed: lymph 9, mixed Darcy 20, cell poroelasticity 5, implicit pore 13, circulation 11 tests (58 total).

## Error-controlled active-wall exchange

`step_with_wall_and_osmotic_laws_adaptive` adds SSPRK2 step doubling with explicit `AdaptiveExchangeConfig` tolerances for relative error, absolute water volume (m3), absolute protein mass (kg), minimum/maximum step and trial count. The normalized discrepancy is `abs(coarse-fine)/3/(absolute+relative*scale)`, checked for every compartment volume/protein mass and every integrated edge water/protein transfer. Edge-ledger checks prevent a balanced loop from concealing pumping errors in nearly unchanged compartment volumes. Two positive conservative half-steps are accepted without extrapolation. Growth uses the cubic-root local-error controller; rejected trials never enter the accepted ledgers. The report distinguishes accepted and rejected steps and records the largest accepted error ratio.

The comparison requires exactly one coarse SSPRK2 step and two fixed half-steps. If internal positivity safeguards subdivide any of these, the adaptive controller rejects the comparison and halves its outer step. This avoids applying the Richardson factor to differently subdivided trajectories or accepting an accidentally identical trajectory. Minimum-step, trial-budget, law or overflow failure rolls back the entire call, even after earlier successful trials. A regression independently proves that a short first trial succeeds and that a longer call with the same one-trial budget leaves the original state unchanged. Local estimates do not guarantee a global error bound; valve switching is nonsmooth, and activation/externally supplied pressure remain constant during each call.

On the independent five-cycle RK4 fixture, maximum allowed step 0.1 s, absolute tolerances 1e-18 m3 and 1e-17 kg:

| Relative tolerance | Accepted / rejected steps across ten phases | Maximum final-state error (m3) | Outlet volume (m3) |
| --- | --- | --- | --- |
| 1e-3 | 75 / 41 | 6.505417849842e-12 | 1.616827912830e-8 |
| 1e-4 | 187 / 33 | 1.005266175276e-12 | 1.617377927997e-8 |
| 1e-5 | 587 / 40 | 9.557841664498e-14 | 1.617468896773e-8 |

RK4 reference outlet volume is 1.617478454615e-8 m3; the strictest demonstrated adaptive setting gives approximately 0.000591 percent relative error in this fixture. All accepted step error ratios were at most one, accepted substep counts exactly twice the accepted adaptive steps, water conserved and uniform protein concentration preserved. This API currently controls the lumped network's time error; the deforming FEM adapter still uses its existing maximum-step SSPRK2 path and does not inherit this adaptive controller automatically.

## Joint adaptive deforming tissue and active lymph exchange

`CellPoreTissue::step_mixed_darcy_with_lymphatic_walls_adaptive` now uses the same shared step-doubling controller as the standalone lymph network. Each coarse/half-step trial owns both tissue and network clones, re-equilibrates FEM at the SSPRK2 stage fluid volumes, recomputes current-geometry RT0 permeability/flux and evaluates external lymphatic wall pressure. Two accepted half-steps commit their joint equilibrated state. Trials, failed nonlinear solves and an exhausted budget leave both original states unchanged; no accepted fluid inventory is paired with a rejected geometry.

`AdaptiveTissueExchangeConfig` adds an absolute nodal position discrepancy tolerance (metres) to the transport tolerances. Error comparison uses maximum Euclidean coarse/fine nodal displacement divided by three and that position tolerance, alongside the existing volume, protein and integrated edge-transfer errors. This geometry tolerance is translation independent. It is a local temporal discrepancy, not a guaranteed final geometry error; mechanical solve tolerance, spatial discretization and possible equilibrium branches still require separate checks. Activation, wall external pressure and material parameters remain prescribed over the call.

The deforming two-cell/four-space capillary-tissue-lymph regression compares against fixed maximum step 0.001 s; further refinement to 0.0005 s changes final geometry by less than 1e-10 m. At unchanged relative transport tolerance 1e-3 and absolute tolerances 1e-18 m3 / 1e-17 kg, tightening the local geometry tolerance from 1e-8 to 1e-10 m changes accepted steps from 3 to 6 (one rejected step each), reducing maximum final position difference from 2.729324186795e-9 to 3.247726514805e-10 m. Thus geometry control independently affects the selected time steps; the local tolerance should not be interpreted as a bound on the accumulated final discrepancy.

Tests also check conservation, exact accepted tissue/network fluid-volume agreement, cached network pressures against recomputed FEM/Biot pressures, and atomic rollback after a successful short trial followed by whole-call budget exhaustion. This closes the previous missing adaptive-time link in the numerical FEM/lymph adapter, but the fixture is not registered whole-body anatomy or a calibrated lymphatic vessel network.

## Adaptive active-lymph exchange on imported ovarian geometry

`cargo run --release -p physics --example anatomy_lymphatic -- --steps=2 --uncoupled-wall --fixed-resistance --output=/tmp/voxy-anatomy-lymph.csv` reproduces the original fixed-external-pressure baseline and runs the joint adaptive RT0/FEM/lymph adapter on the imported left-ovary tetrahedral mesh by default. A positional path selects another validated VXTM mesh; `--fixed` isolates the transport on a fixed skeleton. `--seconds` supplies each phase duration, `--max-step` caps the adaptive step, `--relative-tolerance` and `--position-tolerance` control time discrepancies, `--solid-tolerance` controls FEM convergence, and `--tension` specifies active hoop tension. Odd phases use that tension, even phases zero tension. Defaults are numerical demonstration choices, not physiological measurements.

All actual interior pore faces map to RT0 network edges. A numerical plasma-to-first-cell edge, farthest-cell-to-lymph inlet and lymph-to-plasma return complete the closed network. The capillary uses reflection 0.8 and diffusive conductance 1e-15 m3/s; the lymph inlet/return have ideal valves and no prescribed edge pump head. Active lymph pressure instead follows the current-volume wall law. First/farthest cell selection does not identify real vascular openings, drainage territories or vessel attachments. Plasma and lymph remain lumped compartments, not resolved blood/lymph lumina. The imported ovarian shape has no measured physiological material assignment.

Normal workspace release execution passed on 1718 cells, 462 geometry nodes, 1720 fluid spaces and 3107 exchange edges. Two deliberately short prescribed phases of 1e-5 s each exercise activation switching; they do not constitute validation of a physiological contraction period. At position tolerance 1e-8 m, each phase accepted one adaptive step with zero rejections. Active phase: lymph inlet transfer zero, return transfer 4.179411973074e-17 m3. Relaxed phase: inlet transfer 8.769415419498e-18 m3, return transfer zero. Total water drift after the two phases was -4.235164736272e-22 m3 and protein drift -8.470329472543e-22 kg; maximum displacement 1.229704198710e-6 m.

At stricter position tolerance 1e-11 m, the first phase accepted four steps and rejected two, with maximum accepted normalized discrepancy 0.866965628542; the second accepted one step, discrepancy 0.861531709112. Combined water/protein totals agree with the looser export at its precision: 5.025949880559287e-7 m3 and 5.0309498805592875e-6 kg. Maximum tissue-pressure difference between runs is 3.066472056421077e-4 Pa and maximum node-position difference 3.9065809380625903e-11 m. The stricter run is a numerical comparison, not an exact solution or proof of spatial convergence.

The example exports tissue CSV plus `.network.csv` for plasma/lymph and `.geometry.csv` for accepted/reference node coordinates. Both two-phase exports were independently checked: exact cell/node identity matching, finite pressures/coordinates/masses, positive fluid volumes and nonnegative protein. During verification an unrelated incomplete `liquid/emission.rs` briefly blocked the full build; a source-path-only biomechanics harness ran first. After that file appeared, the normal workspace build and both ordinary example runs passed. No alternative copied implementation was used.

## Interstitial-pressure feedback to lymphatic walls

`LymphaticWallAttachment` explicitly links an external lymphatic compartment to one tissue-cell identity. The new `step_mixed_darcy_with_attached_lymphatic_walls` and adaptive variant evaluate wall external pressure as `wall.external_pressure_pa + current_cell_Biot_pressure` after each FEM equilibrium. The field on the supplied wall is therefore an explicit pressure offset for an attached wall. The final accepted wall pressure is recomputed from the accepted tissue state. Both empty-attachment APIs retain their previous fixed-external-pressure behavior. Invalid target/missing wall/out-of-range cell or duplicate target attachments are rejected atomically.

This represents surrounding interstitial fluid pressure, not the full solid stress tensor or resolved vessel/tissue contact traction. A single-cell mapping does not interpolate a vessel surface or supply measured anatomy. Wall geometry remains lumped and fixed-length, and pressure feedback enters mechanics through fluid exchange/storage rather than a resolved lymphatic cavity traction.

The anatomical driver now enables attachment of lymph compartment 1719 to drainage cell 1575 by default. `--uncoupled-wall` recovers the original fixed-external-pressure comparison; `--external-offset` supplies an explicit offset (default zero). At zero offset, no active tension, and reference wall volume, local tissue pressure and internal wall pressure match; positive fluid entry requires passive recoil or another actual pressure difference. Fixing external pressure to zero instead introduces a different transmural loading whenever tissue pressure is nonzero.

The attached two-phase 1718-cell run passed in the normal release workspace. Active phase return transfer was 5.056564451935e-17 m3 and inlet transfer zero. Relaxed phase return was zero and inlet transfer 2.528282241343e-24 m3; this differs from the original fixed-external-pressure inlet 8.769415419498e-18 m3. Maximum displacement after the two short phases was 1.229706519720e-6 m. Water drift was -4.235164736272e-22 m3 and protein drift -1.016439536705e-20 kg. Independently checking the export reconstructed relaxed wall pressure from drainage-cell pressure and current wall volume with a residual of -3.552713678800501e-15 Pa; all 1718 tissue and two external records were finite with positive fluid volumes and nonnegative protein.

A separate deforming-cell test compares attachments to two different cells, independently reconstructs accepted external/internal wall pressure and inlet flux, checks both total inventories and rejects bad/duplicate links without state changes. Neither this numerical comparison nor the anatomy-shaped demonstration validates physiological lymph flow rates or contraction periods.

## Current-radius lymphatic tube resistance

`LymphaticHydraulicAttachment` links an exchange edge to a wall compartment at one of that edge's endpoints, supplying a tube segment length and positive dynamic viscosity. The current radius comes from accepted/trial fluid volume and fixed wall length. Tube resistance is `8*mu*segment_length/(pi*r^4)`, added in series with the base resistance `1/edge.hydraulic_m3_per_pa_s`. Multiple endpoint-wall segments may contribute to one edge; duplicate edge/compartment pairs are rejected. The original edge metadata remains the fixed resistance; accepted rate diagnostics include the actual radius-dependent resistance.

`step_mixed_darcy_with_lymphatic_geometry` and its adaptive variant recompute this resistance at both SSPRK2 stages alongside nonlinear protein osmosis, interstitial wall-pressure feedback and current-geometry RT0 tissue transport. Internal tissue faces stay under RT0 control. The adjusted flow also changes donor protein advection; diffusive protein conductance and valve closure retain their own laws. Linked rates are staged before replacement so invalid later attachments cannot partially alter the input.

This is a circular Newtonian laminar Poiseuille segment at fixed length with no-slip walls and a quasisteady resistance. It does not resolve entrance losses, unsteady fluid inertia, noncircular collapse, lymph cell rheology or actual valve geometry. Segment lengths and fixed interface resistances must be explicitly assigned; no anatomical vessel radius or measured viscosity is inferred.

The anatomical driver now adds half the wall length to each inlet/return edge at viscosity 0.001 Pa s. `--fixed-resistance` disables these tube segments for the earlier baseline; `--lymph-interface-resistance` controls each remaining fixed series resistance (default 1e13 Pa s/m3). At the default 1e-9 m3 wall volume and 0.001 m length, radius is approximately 0.564 mm and each half-segment resistance approximately 1.257e7 Pa s/m3; thus the original default interface resistance dominates and the tube effect is small. This hierarchy is a demonstration input, not a fitted physiological conclusion.

Independent tests verify forward/reverse/zero flow, additive diffusion, exact inverse-fourth-radius scaling and atomic invalid-link rejection. A deforming-tissue test recomputes the accepted Poiseuille-plus-interface valve flows and protein rates from accepted pressure and radius, checks both inventories and rejects a tube link targeting an internal pore edge. The default 1718-cell two-phase anatomical run passed with finite outputs and conserved joint inventories; its active return was 5.056558156219e-17 m3, and relaxed inlet 2.528275911628e-24 m3.

A second normal-release anatomical run with `--steps=2 --lymph-interface-resistance=1e8` exercised appreciable tube contribution and faster pressure exchange. The active phase accepted 49 adaptive steps and rejected four; return transfer was 5.057538915395e-13 m3, inlet zero. Relaxation accepted one step; inlet transfer 2.241967944720e-15 m3 and return zero. Capillary transfers were +4.286668699801e-17 and +4.829862217938e-17 m3 in the two phases. Final water drift was -6.352747104407e-22 m3 and protein drift -1.101142831431e-20 kg; maximum displacement 1.229903054171e-6 m. Both radius-enabled exports (default and reduced fixed resistance) were independently checked for 1718 finite tissue records, two external compartments, 462 finite geometry nodes, positive fluid volumes, nonnegative protein and the accepted interstitial-plus-wall pressure equation. This parameter change is a numerical sensitivity demonstration, not physiological calibration or a measured pumping result. Focused release tests passed: lymph 11, mixed Darcy 23, cell poroelasticity 5, implicit pore 13, circulation 11 (63 total).

## Applicability of quasisteady lymph resistance

`LymphaticHydraulicAttachment::diagnostics` evaluates explicit density, signed flow and forcing period at the supplied wall volume. It reuses the circulation solver's rigid-pipe resistance and inertance: `R = 8*mu*l/(pi*r^4)`, `L = rho*l/(pi*r^2)`. It reports velocity `Q/(pi*r^2)`, Reynolds number `2*r*rho*abs(velocity)/mu`, Womersley number `r*sqrt(omega*rho/mu)`, relaxation time `L/(R+R_fixed)` and `omega*L/(R+R_fixed)`. These are model diagnostics, not an implementation of unsteady lymph flow or a guarantee of validity. Large Womersley numbers also challenge the assumed fully developed quasisteady velocity profile; a lumped inertance alone does not resolve that profile.

The anatomical driver takes `--lymph-density=` (default synthetic input 1000 kg/m3). Its diagnostics use the explicit two-phase forcing period `2*seconds` and each phase's integrated exchange divided by phase duration. Radius is evaluated at the accepted phase endpoint. Thus printed Reynolds numbers are phase-average-flow proxies, not measured instantaneous maxima. The period is not inferred from the adaptive integration step. Abrupt switching of tension contains faster transients than this fundamental-period estimate.

Verified normal-release command: `cargo run --release -p physics --example anatomy_lymphatic -- --steps=2 --lymph-interface-resistance=1e8 --output=/tmp/voxy-anatomy-lymph-regime.csv`. In the active return segment, the diagnostics gave radius 5.640468949654e-4 m, velocity 5.060098079691e-2 m/s, Reynolds 57.08265220140, Womersley 316.1477891918, inertance 5.002530049040e5 Pa s2/m3, relaxation time 4.443569399695e-3 s and inertial/resistive ratio 1395.988498180. Consequently this fast, reduced-resistance fixture cannot justify neglecting liquid inertia. Its conservation and adaptive convergence establish behavior of the quasisteady numerical model, not physical pumping accuracy. Low Reynolds or zero phase flow alone does not establish that transient inertia is negligible.

An independent analytic test checks all coefficients, signed velocity, zero-flow/high-frequency behavior, the effect of added series resistance and invalid density/period rejection. The remaining implementation requirement is an inertial, conservative lymph transport law coupled to changing wall volume and tissue pressure, with independent transient reference solutions and time convergence; these diagnostics do not satisfy that requirement.

## Inertial lymph segment response

`LymphaticHydraulicAttachment::momentum_response` supplies the trial-geometry backward-Euler flow and pressure tangent for a circular segment with explicit density and series resistance. It reuses the circulation momentum response, including ideal-valve complementarity. For an open linear segment, `L*(Q_new-Q_old)/dt + (R_pipe+R_fixed)*Q_new = pressure_drive`; the returned tangent is `1/(R_pipe+R_fixed+L/dt)`. The caller must supply the full signed pressure drive before valve clipping, including osmotic and wall contributions. Forward momentum may persist temporarily against reversed pressure; the valve closes when the discrete unconstrained flow would become negative.

This is a constitutive response for a future simultaneous inventory/pressure solve, not a new network stepping mode. Existing anatomical lymph runs still use quasisteady transport. No old clipped valve rate can reconstruct the signed drive required here. Trial radius dependence must be included by the coupled solver, and the accepted flow history must commit atomically with water, protein and FEM geometry. Constant-coefficient tests independently check the momentum residual and tangent, first-order convergence to `Q(t)=Q_steady*(1-exp(-t*R/L))`, retained forward momentum during pressure reversal, valve closure and invalid time/flow rejection. Plug-flow inertance plus Poiseuille resistance still does not resolve a high-Womersley velocity profile.

## Simultaneous inertial wall-network inventories

`LymphNetwork::step_with_inertial_walls` now solves one backward-Euler step by simultaneous fixed-point iteration of water volume, protein mass, wall pressure, nonlinear osmosis and radius-dependent segment momentum. An explicit mutable per-edge flow history carries accepted momentum between calls. Each linked edge has one segment and retains its fixed series resistance; unlinked edges use their original osmotic exchange law. The signed pressure drive is evaluated before valve clipping. Protein advection uses the accepted-flow donor concentration; closed ideal valves also close diffusion.

Each trial reconstructs all inventories from the original state with opposite signed transfers. The converged state is the state at which pressure, momentum and protein flux were evaluated; backward-Euler inventory residuals are bounded by the supplied absolute volume/protein tolerances. Integrated edge ledgers can therefore differ from actual inventory changes by these nonlinear residuals, rather than being bitwise identical. Total inventories are conserved to summation roundoff. Geometry, pressure/rate caches and caller flow history commit only together after success; invalid trials or exhausted iterations leave all original state unchanged.

The method has no automatic time adaptation or stiff convergence guarantee. Too-large steps can fail the positivity or fixed-point checks and must be retried from the unchanged state with a smaller step. It does not yet call the tissue FEM equilibrium or RT0 adapter and is not selected by the anatomical driver. Density, convergence tolerances and iteration limits are explicit numerical inputs. The independent two-space test reconstructs the accepted momentum equation with current radius, backward-Euler water/protein inventory residuals, donor flux and both total balances, and checks atomic rejection including flow history after iteration exhaustion.

## Inertial lymph coupled to deforming RT0/FEM tissue

`CellPoreTissue::step_mixed_darcy_with_inertial_lymphatic_geometry` connects the simultaneous backward-Euler network solve to the existing equilibrated FEM pressure law and current-geometry RT0 transport. Every nonlinear trial sets tissue cell fluid inventory, equilibrates the staged solid, evaluates local Biot pressures and attached wall external pressure, and recomputes reference-permeability push-forward and pore-face flux. Lymph momentum uses the signed osmotic pressure drive and current radius; the RT0 callback preserves those external inertial rates while replacing only internal pore-face rates. The network's shared callback core retains the wall-only API.

The accepted geometry is the geometry at which accepted network pressure and flow were evaluated. Its cell fluid inventories match the network trial exactly. Tissue, network and caller flow history commit together; all fallible operations precede commit. Explicit laws, wall array, density, absolute nonlinear inventory tolerances and iteration budget are required. One segment per linked edge is currently supported; additional series segments and automatic time/geometry adaptation remain absent. The anatomical example still uses the adaptive quasisteady path until the inertial mode has been exercised on that larger mesh.

A separate two-deforming-tetrahedra test independently reconstructs actual accepted FEM/Biot pressure, attached wall pressure, both valve momentum flows with current radius and density, both total inventories, and rollback of geometry, water, protein and history after iteration exhaustion. It passed in normal release mode. The existing 14 lymph and 23 mixed-Darcy tests passed after callback refactoring; the added inertial FEM test passed separately (38 tests across those suites).

## Inertial mode on imported organ mesh

The anatomical driver now accepts `--inertial` and `--inertial-substeps=N` (positive integer, default one). This mode advances each explicit active/relaxed phase using N fixed backward-Euler substeps, preserving momentum history across the phase boundary. It uses the joint RT0/FEM inertial adapter with explicit density, nonlinear inventory tolerances 1e-20 m3 and 1e-19 kg, and at most 1000 iterations per substep. `max_error_ratio=not_estimated` explicitly identifies the absence of adaptive error estimation; `--max-step` pertains to the alternative adaptive quasisteady path. No physical contraction period is inferred.

Verified normal-release command on 1718 left-ovary tetrahedra: `cargo run --release -p physics --example anatomy_lymphatic -- --inertial --steps=2 --lymph-interface-resistance=1e8 --output=/tmp/voxy-anatomy-inertial.csv`. At one substep per phase, active return transfer was 9.893435399690e-15 m3, versus the earlier quasisteady 5.057538915395e-13 m3. Relaxed return was zero at this coarse step. This comparison alone does not establish time accuracy.

Refinement to 2, 4, 8 and 16 substeps per phase produced active return transfers 7.511236022770e-15, 6.286384071827e-15, 5.667336559511e-15 and 5.356398171676e-15 m3. Successive active-transfer changes approximately halved, consistent with first-order time refinement. Relaxed return transfers were 3.968409739105e-16, 1.566644713591e-15, 2.151581598934e-15 and 2.443810260308e-15 m3: retaining momentum and resolving the valve transition matters even after tension switches off. Combined return over both phases changed from 7.818918158445e-15 to 7.800208431984e-15 m3 between 8 and 16 substeps (about 0.24%). Individual phase transfers and geometry are not thereby proven converged, and no accumulated-error bound is claimed.

The runs passed joint-inventory assertions. Independent CSV inspection for the 1/2/4-substep exports checked 1718 tissue records, two external records and 462 geometry nodes for finite fields, positive fluid volumes and nonnegative protein, reconstructed the final attached passive-wall pressure with zero reported residual, and recovered total water 5.025949880559287e-7 m3 and protein 5.0309498805592875e-6 kg. At 16 substeps the reported final water drift was -3.176373552204e-22 m3 and protein drift -1.270549420881e-20 kg. These are numerical demonstrations with synthetic ports, tension and microsecond phases; high Womersley profile dynamics, resolved vessel wall traction and physiological calibration remain unimplemented.

## Independent transient reference for nonlinear inertial transport

`inertial_nonlinear_network_converges_to_independent_rk4` tests the full wall-network trajectory against a separately coded three-state ODE, without calling production wall pressure, pipe coefficient, osmotic rate or time-integration functions for its reference. The ODE carries volume, protein mass and flow; opposite compartment inventories follow conservation. It includes exponential/collapse wall pressure, changing radius-dependent resistance and inertance, linear/quadratic protein osmosis, partial reflection, donor advection and protein diffusion. A 0.1-second synthetic trajectory is resolved by independent RK4 with 10000 and 20000 steps; all final states agree within 2e-13 in their respective SI units.

Normal-release results for the production backward-Euler network at 20/40/80 steps: final volume errors were 1.620488787080e-5, 8.119182269284e-6 and 4.062815546990e-6 m3; protein errors 2.252865954766e-5, 1.128808768369e-5 and 5.648644496814e-6 kg; flow errors 4.393508528164e-4, 2.201079215268e-4 and 1.101369072673e-4 m3/s. Each error approximately halves under refinement, verifying first-order convergence of all three coupled state variables for this nonlinear fixture. The test independently checks total water/protein conservation and integrated edge-transfer consistency within nonlinear residual tolerances throughout the trajectory.

This is a numerical unit-scale fixture, not measured lymph geometry or composition. It validates the wall-network time integration, not the anatomical FEM trajectory, resolved high-frequency velocity profile or physiological calibration. Valve transitions are covered by separate momentum/closure tests, not by this smooth forward-flow reference.

## Adaptive inertial wall-network time integration

`step_with_inertial_walls_adaptive` stages network and per-edge flow history together and uses backward-Euler step doubling. The shared adaptive controller now explicitly selects integration order: existing SSPRK2 callers retain Richardson divisor 3 and cubic-root growth; the inertial first-order path uses divisor 1 and square-root growth. It accepts two half-steps without extrapolation and controls each water/protein inventory, integrated edge transfer and final flow history. Flow tolerance is explicit in m3/s and includes the selected relative tolerance. Whole-call failure, including trial exhaustion after accepted work, commits neither inventory nor momentum.

The nonlinear iteration budget and absolute nonlinear inventory tolerances are explicit, separate from local time-error tolerances. Recoverable inertial iteration-limit or trial-inventory failures now halve the trial step; invalid constitutive inputs and other failures still return atomically. This controller is not a robust stiff nonlinear solver or a rigorous global-error bound. The FEM adapter and anatomical driver still use their explicit fixed inertial substeps; they do not yet select this wall-network adaptive API or control coupled geometry/momentum error together.

The independent nonlinear RK4 reference test also covers the adaptive path. Relative time tolerances 1e-3 and 1e-4 (absolute inventory tolerances 1e-10 m3/kg, flow tolerance 1e-8 m3/s, nonlinear tolerances 1e-14 m3/kg) accepted 746 and 2137 double-step trials, with three error rejections each. Final volume errors were 7.735989387969e-7 and 1.156350162290e-7 m3; protein errors 1.074810375590e-6 and 1.606911776975e-7 kg; flow errors 2.089539350682e-5 and 3.126117116406e-6 m3/s. Tighter tolerances reduced all errors by about 6.7 times for this fixture. Tests check accepted error ratios, exactly two integration substeps per accepted trial, conserved totals and atomic network/history rollback on trial-limit failure.

## Joint adaptive inertial FEM/lymph control

`step_mixed_darcy_with_inertial_lymphatic_geometry_adaptive` stages `(CellPoreTissue, LymphNetwork, flow_history)` together. Its first-order step-doubling estimate controls each fluid/protein inventory and edge ledger, Euclidean node-position differences in metres and per-edge momentum history in m3/s. First-order geometry/flow discrepancies use no Richardson divisor of three. Accepted two-half-step states include their own equilibrated FEM and RT0 transport. Failure commits none of the three states. Recoverable inertial iteration-limit or trial-inventory failures now retry on staged copies at half the step; other nonlinear/FEM failures propagate atomically.

The driver selects this mode with `--inertial-adaptive`; `--max-step`, `--relative-tolerance` and `--position-tolerance` now govern joint inertial time control. Flow absolute tolerance is currently 1e-12 m3/s, with nonlinear volume/protein tolerances 1e-20 m3 and 1e-19 kg. These are explicit numerical settings in the driver, not physiological measurements.

Verified normal-release command: `cargo run --release -p physics --example anatomy_lymphatic -- --inertial-adaptive --steps=2 --lymph-interface-resistance=1e8 --output=/tmp/voxy-anatomy-inertial-adaptive.csv`. On the imported 1718-cell mesh, active phase accepted 54 trials and rejected two, maximum accepted error ratio 0.8895097172095; relaxed phase accepted 46 and rejected three, maximum ratio 0.8128757999658. Return transfers were 5.091297399204e-15 and 2.721449541751e-15 m3; relaxed inlet transfer 3.618907383329e-20 m3. Final total water drift was -4.235164736272e-22 m3 and protein drift -6.776263578034e-21 kg, maximum displacement 1.229733124784e-6 m. Independent export checks confirmed all 1718 tissue/two external/462 geometry records finite, positive volumes, nonnegative protein and the accepted attached-wall pressure equation. This local-error-controlled numerical result is not a physiological calibration or proof of global accuracy.

A new deforming-tissue test checks accepted error bounds, exactly two half-steps per accepted trial, both inventory totals, actual FEM/Biot and attached-wall pressures, accepted rate/history agreement and atomic geometry/inventory/cache/history rollback on trial exhaustion. Normal-release suites passed: lymph 15, mixed Darcy 25, circulation 11 (51 total).

## Retrying recoverable inertial nonlinear trials

The first-order adaptive controller now retries two explicitly identified recoverable failures: `inertial lymph iteration limit` and `invalid inertial lymph trial inventory`. It discards the failed coarse/fine trial copies, increments the rejection count and halves the proposed time step. Network, FEM geometry and flow history remain staged throughout. A minimum-step failure returns the original subsolve reason; trial-budget exhaustion remains atomic. Invalid density, geometry, wall laws, overflow and FEM equilibrium errors are not masked by retries. The existing second-order SSPRK2 path keeps its prior error handling.

A regression deliberately limits the wall-network solve to two nonlinear iterations. A direct 0.01-second step fails with iteration exhaustion and preserves the starting state. The adaptive call succeeds after five rejected attempts and 70 accepted double-step trials, with conserved total water/protein and positive forward flow. A subsequent forced failure at the minimum step preserves volumes, protein, pressure/rate caches and accepted flow history exactly; negative density returns its original invalid-input error immediately. This improves recoverability, not the physical validity of the constitutive model.

## Resolved radial unsteady velocity profile

`lymph::profile::RadialPipe` introduces annular finite-volume backward-Euler integration of `rho*du/dt = pressure_gradient + mu*(1/r)*d(r*du/dr)/dr` in a rigid circular tube. The axis has zero radial viscous flux; the stationary outer wall is no-slip. Ring areas provide exact discrete cross-sectional integration; shared radial viscous conductances conserve internal momentum exchange. An O(N) tridiagonal solve returns the signed integrated flow, its gradient tangent, kinetic energy per length and viscous dissipation per length. Failed steps preserve all radial velocity history. Radius, density, viscosity and resolution are explicit.

The pressure-driven unsteady circular-flow basis is described by [Womersley (1955), Journal of Physiology](https://pmc.ncbi.nlm.nih.gov/articles/PMC1365740/). This implementation directly discretizes the radial diffusion equation; it is not an implementation of that paper's analytic Bessel expression. It does not yet replace the lumped inertance in the network/FEM APIs, include wall radial motion, axial convection, entrance flow, valves or non-Newtonian rheology. Oscillatory amplitude/phase validation against an independent Womersley solution and adequate near-wall resolution at large Womersley number remain required before high-frequency accuracy can be claimed.

A normal-release test checks stationary convergence to the independent Poiseuille flow `pi*r^4*gradient/(8*mu)` with radius/density/viscosity/gradient all one. At 8/16/32 rings, errors were 0.006135923151543, 0.001533980787886 and 0.000383495196971 m3/s, decreasing by four on each radial refinement. Each step also satisfies the backward-Euler energy inequality: pressure work bounds the kinetic increment plus viscous dissipation. Separate checks cover signed forcing symmetry, the linear flow tangent, transient flow below its steady value, persistent forward momentum and declining energy after drive removal, and atomic invalid-time rejection. These checks establish the stationary spatial limit and discrete dissipativity, not oscillatory phase accuracy or whole-body physiology.

## Independent oscillatory Womersley reference

`radial_pulsatile_flow_refines_to_independent_womersley_amplitude_and_phase` compares the radial finite-volume solver to the analytic circular-tube harmonic flow response. Its independent complex Bessel J0/J1 power series evaluates `Q_hat = pi*R^2*G_hat/(i*omega*rho) * (1 - 2*J1(z)/(z*J0(z)))`, where `z^2=-i*omega*rho*R^2/mu`. It does not call production radial flow coefficients or numerical integration. The source formulation is [Womersley (1955)](https://pmc.ncbi.nlm.nih.gov/articles/PMC1365740/). Signed cosine pressure forcing is advanced for forty periods from rest; final-period Fourier projections recover real/imaginary flow response and phase lag.

With unit radius/density/viscosity and Womersley numbers 2 and 10, joint spatial/time refinement uses 32/64/128 rings and 128/256/512 samples per period. At alpha=2 the relative complex-response errors are 1.258304976221%, 0.649201741217% and 0.329758142120%; phase errors 0.007948529531, 0.003936269671 and 0.001958595315 radians. At alpha=10, errors are 2.444531107296%, 1.183182347309% and 0.581979663205%; phase errors 0.024443458660, 0.011825462702 and 0.005809332942 radians. Tests require refinement improvement and finest relative response error below 1.5%, with lagging analytic flow. This demonstrates joint refinement in those two regimes; it does not independently isolate radial and temporal error orders.

The anatomical fast fixture has alpha approximately 316, outside this validated range. No accuracy claim for that profile regime, moving walls, valve interfaces or whole-body coupling follows from these tests. The radial solver still runs independently of the deforming FEM/lymph network.

## Radial-profile pipe with a series hydraulic interface

`RadialPipe::step_with_series_resistance` couples the resolved radial momentum history to an explicit massless linear interface resistance. It accepts signed endpoint pressure difference, physical pipe length and nonnegative interface resistance. For a fixed old profile, the implicit pipe response is `Q=Q_free+M*gradient`, with gradient mobility M from the tridiagonal solve. The method solves `gradient=(DeltaP-R_interface*Q_free)/(length+R_interface*M)` directly, retaining the accepted full velocity profile rather than fitting a fresh Poiseuille profile. It returns the endpoint pressure tangent `M/(length+R_interface*M)` alongside the original pipe report; the report's tangent retains pressure-gradient units.

Solving for the gradient directly avoids cancellation from subtracting nearly equal endpoint pressure and interface pressure loss in an interface-dominated pipe at rest. All trial operations use clones, and the final profile step remains atomic. This interface is bidirectional and linear; it does not implement valve complementarity, leakage or leaflet motion, moving radius, protein/water inventories or network/FEM coupling.

Tests independently reconstruct interface pressure balance and the accepted radial profile, verify the endpoint tangent by finite differences, retained forward momentum after drive removal, steady convergence to `1/(8*mu*length/(pi*r^4)+R_interface)`, invalid-resistance rollback and an extreme 1e20 Pa s/m3 interface without pressure-cancellation loss. The composite backward-Euler energy check includes both pipe viscous loss and `R_interface*Q^2` dissipation, bounded together with kinetic-energy change by endpoint pressure work.

## Ideal valve with resolved radial momentum history

`RadialPipe::step_with_ideal_valve` adds ideal forward-valve complementarity to the rigid radial profile plus massless series resistance. It first evaluates the unrestricted implicit flow on a copy. Nonnegative flow accepts the open profile and its endpoint tangent. Negative unrestricted flow selects a zero-net-flux constraint: `gradient=-Q_free/M`. The returned nonnegative valve reaction is `length*gradient-DeltaP`; the closed endpoint tangent is zero. Interior velocities are solved with that constraint pressure, not cleared. Net zero flux holds to tridiagonal roundoff; no clipping of individual velocities hides shear or kinetic history.

This is an instantaneous ideal constraint, not a resolved closure transient. It has no leaflet inertia, elasticity, leakage, finite gap, fluid compressibility or pressure wave propagation. The radial profile may contain oppositely signed local axial velocities while its integrated flux is zero; it remains the fully developed fixed-radius idealization and does not resolve the flow near a real closed leaflet. The API is not yet integrated into network water/protein exchange or the anatomical FEM driver.

A regression builds a steady moving profile, checks persistent forward flow under a small reversed pressure, then closes under a sufficiently strong reversal. It independently reconstructs the constraint gradient, pressure reaction and complete accepted radial profile; verifies zero net flow with nonzero stored kinetic energy, energy decay including viscous loss, continued decay on a subsequent closed step, reopening under forward pressure and atomic invalid-input rejection. The closed constraint does no net hydraulic work at zero flow; backward-Euler and viscous losses dissipate the retained history.

## Resolved radial profiles coupled to network inventories

`LymphNetwork::step_with_radial_profiles` now jointly solves backward-Euler compliant storage, nonlinear protein osmosis, donor advection/diffusion and fixed-radius radial pipe momentum. `RadialExchange` supplies an edge identity, physical conduit length and explicit `RadialPipe` history. Each linked edge retains its base hydraulic resistance as a massless series interface; the resolved profile supplies pipe response. Ideal one-way edges use the constrained radial valve; closed net flow is represented as exactly zero in the inventory ledger while the profile retains linear-solve roundoff. Closed valves also block protein diffusion. Unlinked exchanges keep their prior osmotic law.

Every nonlinear iteration restarts radial advancement from the original accepted profile, not from the previous nonlinear trial. Only converged profiles and the network commit together. The accepted inventory residual is bounded by explicit absolute water/protein tolerances, as in the inertial wall solver. Invalid mapping, failed profile solve, positivity failure or iteration exhaustion preserves inventory, caches and all radial histories. Pipe radius is fixed and independently specified; compliant compartment volume does not imply a changing conduit radius. This API does not yet supply FEM pressure callbacks, active wall laws, automatic time adaptation or a moving-wall radial equation.

A three-step regression independently reconstructs each accepted signed osmotic pressure drive, series pressure balance, full radial profile from the original history, donor/diffusive protein flux and backward-Euler inventories, verifies both totals and atomic late iteration-limit failure. A separate closed-valve case confirms zero water and protein transfer despite unequal concentrations and nonzero diffusive conductance. These are numerical coupling tests, not anatomical or physiological validation.

## Radial-profile network coupled to deforming tissue

`CellPoreTissue::step_mixed_darcy_with_radial_profiles` supplies staged FEM pressure and current-geometry RT0 flux callbacks to the resolved-profile inventory solver. Every trial updates local tissue fluid inventories, equilibrates the solid, evaluates Biot pressure and optional external-pressure feedback to active lymph walls, and reconstructs pore-face transport. External pipe rates retain their resolved velocity profile and ideal-valve state; only internal tissue faces are replaced by RT0 flux. A radial pipe mapping targeting an internal Darcy face is rejected.

Accepted tissue, network pressure/rate caches and every radial profile commit together. All fallible operations precede commit. The conduit radius is independently supplied and fixed even if the connected compliant reservoir/wall volume changes. Thus this coupling does not implement moving-wall radial fluid mechanics or equate reservoir geometry with rigid conduit geometry. Automatic joint radial-profile time control and use in the anatomical driver remain absent; that driver still selects lumped inertance for its inertial modes.

A deforming-tissue regression independently reconstructs accepted FEM/Biot and attached-wall pressure, each external pipe's signed osmotic drive, ideal-valve series response and full radial velocity vector from its original history. It checks both joint inventories, complete rollback of geometry/inventory/cache/profile state on iteration exhaustion and rejection of a pipe overriding an internal RT0 edge. Existing wall-only radial tests continue through the shared callback core.

## Adaptive FEM/radial-profile time control

`step_mixed_darcy_with_radial_profiles_adaptive` stages tissue, network and all radial pipes together. First-order step doubling controls each fluid/protein inventory, integrated exchange ledger, Euclidean node-position discrepancy and every ring's axial velocity discrepancy. The latter uses an explicit absolute tolerance in m/s plus relative scaling of the coarse/fine ring velocities, so coincident integrated flow cannot hide a differing internal profile. Accepted states are two half-steps without extrapolation; rejection discards all trial histories. Recoverable network iteration/positivity failures use the existing step-halving path.

A deforming-tissue test verifies accepted error ratio at most one, two integration substeps per accepted trial, both conserved inventories, actual accepted FEM/Biot and attached wall pressure, finite radial histories and complete geometry/inventory/cache/profile rollback after whole-call trial exhaustion. Its regular controls accepted two trials with maximum error ratio 0.034563890992867. To isolate profile control, the same starting state was rerun with zero relative tolerance, loose inventory/position budgets and absolute velocity tolerances 1e-8 and 1e-12 m/s. The former accepted two trials without rejection; the latter accepted 339 and rejected eight. The test requires the tighter profile tolerance to increase accepted step count. This establishes controller response, not a separate independent global trajectory-error bound.

Conduit radius remains fixed. The anatomical driver does not yet select this radial mode; its existing inertial modes still use lumped momentum. No adaptive radial spatial resolution, moving-wall momentum transport, resolved leaflet or calibrated anatomy has been added by this change.

## Radial-profile mode on the imported organ mesh

The anatomical driver now accepts `--radial-adaptive`, `--radial-rings=N` (default 256) and `--radial-velocity-tolerance=` (default 1e-7 m/s). It explicitly constructs fixed-reference-radius conduit profiles from the wall reference volume and supplied segment lengths, viscosity and density. These profiles couple to active compliant reservoir pressure, attached tissue FEM pressure, RT0 transport and selective protein exchange; conduit radius stays fixed when reservoir volume changes. Printed tube diagnostics use the fixed reference radius in this mode. Combining this mode with `--fixed-resistance` is rejected because that flag removes explicit conduits.

Normal-release command: `cargo run --release -p physics --example anatomy_lymphatic -- --radial-adaptive --radial-rings=256 --steps=2 --lymph-interface-resistance=1e8 --output=/tmp/voxy-anatomy-radial-256.csv`. The 1718-cell run accepted 56/66 trials in active/relaxed phases, rejecting 3/4. Return transfers were 5.060572859297e-15 and 2.679934583594e-15 m3; final water/protein drifts -1.376428539288e-21 m3 and -1.101142831431e-20 kg. Maximum displacement was 1.229736938210e-6 m.

A second terminal-successful run at 512 rings accepted 60/77 trials, rejecting four in each phase. Return transfers were 5.056595742206e-15 and 2.681924202884e-15 m3; final water/protein drifts -1.058791184068e-22 m3 and -1.270549420881e-20 kg; maximum displacement 1.229735424621e-6 m. The combined return differs by approximately 0.025683% between these runs. Their adaptive time meshes differ, so this is a joint numerical sensitivity comparison, not isolated radial-order convergence or an independent validation at Womersley approximately 316.

Independent inspection of both exports confirmed 1718 finite tissue records, two finite external records, 462 finite geometry nodes, positive fluid volumes, nonnegative protein and the accepted attached-wall pressure equation. Profiles themselves are not exported by this driver yet. Fixed radius, synthetic vessel identities/ports, abrupt microsecond tension phases and missing resolved leaflet/wall dynamics still preclude claims of physiologically calibrated lymph pumping.

## Frequency-domain validation at high Womersley number

`RadialPipe::harmonic_response` solves the same annular viscous operator with complex inertia `i*omega*rho*area`, returning the per-ring complex axial velocity and integrated complex flow for a real cosine pressure-gradient amplitude. It uses an O(N) complex tridiagonal elimination, performs no startup integration and leaves accepted time-domain velocity history untouched. This separates radial spatial error from backward-Euler time error for a stationary harmonic forcing. It does not supply moving geometry or nonlinear valve harmonics.

An independent analytic test evaluates the Womersley integrated-flow expression using the backwards continued fraction `J_n/J_(n-1)=z/(2*n-z*J_(n+1)/J_n)`, avoiding cancellation of the complex J0/J1 power series at large alpha. Continued-fraction truncations at 1000 and 2000 terms agree within 1e-13 relative flow norm for the tested cases. The test uses no production annular coefficients or tridiagonal solver for its reference. The existing low-alpha time-domain reference uses a separate Bessel power-series construction.

Normal-release harmonic relative complex-flow errors for 256/512/1024 rings: alpha=2, 1.412151354923e-5 / 3.530377484742e-6 / 8.825936666169e-7; alpha=10, 4.092186347829e-5 / 1.023042321522e-5 / 2.557603128843e-6; alpha=sqrt(100000)=316.227766016838, 1.153866822648e-3 / 3.013308824465e-4 / 7.554960871859e-5. Each radial refinement reduces error approximately fourfold. At the high-alpha point the finest relative integrated harmonic response error is about 0.007555%.

This validates the integrated stationary harmonic response of the fixed-radius radial discretization in these regimes. It does not independently validate every ring's pointwise profile error, the anatomical transient integration, nonlinear closure phases, deformation coupling or physiology. The earlier 256/512-ring anatomical sensitivity comparison used adaptive time steps and remains a different, weaker type of evidence.
