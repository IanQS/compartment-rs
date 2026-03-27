# compartment-rs — Implementation Plan

## Status Legend

- `[ ]` not started
- `[/]` in progress
- `[x]` done

---

## Channels (`src/channels.rs`)

Foundation for all voltage-gated ion channel math. Everything else depends on this.

- [x] Define `ChannelType` enum (`Sodium`, `Potassium`, `Leak`, `Unspecified`)
- [x] Define `Sodium`, `Potassium`, `Leak` structs wrapping a shared `Channel`
- [ ] Define `ChannelDynamics` trait skeleton
- [x] Fix `Channel` struct — split compartment-level vs channel-level fields:
  - [x] Keep per-channel: `g_bar` (max conductance), `e_rev` (reversal potential)
  - [x] Move to compartment: `capacitance`, `axial_resistivity`
- [ ] Fix `ChannelDynamics` trait — add return types and split rate constants:
  - [ ] `conductance(voltage: f64, ...) -> f64`
  - [ ] `alpha(voltage: f64) -> f64` (forward rate constant)
  - [ ] `beta(voltage: f64) -> f64` (reverse rate constant)
- [/] Implement `ChannelDynamics` for `Sodium`:
  - [x] `alpha_m`, `beta_m` (activation gate, `m`)
  - [x] `alpha_h`, `beta_h` (inactivation gate, `h`)
  - [x] `conductance` → `g_bar_Na * m^3 * h`
- [/] Implement `ChannelDynamics` for `Potassium`:
  - [x] `alpha_n`, `beta_n` (activation gate, `n`)
  - [x] `conductance` → `g_bar_K * n^4`
- [/] Implement `ChannelDynamics` for `Leak`:
  - [x] `conductance` → `g_bar_L` (constant, no gating variables)
- [/] Unit tests:
  - [x] Test α/β values at known voltages against HH literature values
  - [ ] Test conductance outputs for known gating variable states

---

## Dynamics (`src/dynamics.rs`)

Integrates the HH equations forward in time using the channels above.

- [x] Define `CellularDynamics` enum (`HodgkinHuxley`, `Passive`, `Extracellular`)
- [x] Define `Dynamics` trait skeleton (`propagate`, `update`)
- [ ] Refine `Dynamics` trait — replace vague `propagate`/`update` with:
  - [ ] `step(&mut self, dt: f64, i_ext: f64)` — advance one timestep
  - [ ] `membrane_potential(&self) -> f64` — read current `V_m`
- [ ] Flesh out `HodgkinHuxley` struct — add state variables:
  - [ ] `v_m: f64` (membrane potential)
  - [ ] `m: f64` (Na activation gate)
  - [ ] `h: f64` (Na inactivation gate)
  - [ ] `n: f64` (K activation gate)
- [ ] Implement `Dynamics::step` for `HodgkinHuxley`:
  - [ ] Sum ionic currents: `I_Na + I_K + I_L` using conductances from channels
  - [ ] Compute `dV/dt = (I_ext - I_ionic) / C_m`
  - [ ] Update gating variables via forward Euler: `dm/dt = alpha_m*(1-m) - beta_m*m`, etc.
  - [ ] Advance state by `dt`
- [ ] Implement `Dynamics::step` for `Passive`:
  - [ ] Only leak current: `dV/dt = (I_ext - g_L*(V - E_L)) / C_m`
- [ ] Unit tests:
  - [ ] Test that `HodgkinHuxley` produces an action potential under a suprathreshold step current
  - [ ] Test that gating variables converge to steady-state at resting potential (`~-65 mV`)
  - [ ] Test `Passive` membrane charges and discharges with correct time constant `τ = R_m * C_m`

---

## Branching / Compartment Refinement (`src/compartments.rs`)

The correct pipeline is: **SWC points → coalesce into sections → d_lambda rule → compartments**.
Currently each SWC point becomes its own "compartment" before the d_lambda rule, which is incorrect — the rule needs to operate on whole cable sections (contiguous, unbranched stretches), not individual sample points.

- [x] Build `Compartments` from topologically sorted SWC nodes
- [x] Compute compartment length from 3D coordinates
- [x] `d_lambda_rule` — subdivide compartments to satisfy spatial accuracy criterion
- [x] **Branch coalescing** (prerequisite for correct d_lambda application):
  - [x] Define a `Section` or `Branch` struct: contiguous run of SWC points with no forks, characterized by total length and mean diameter
  - [x] Implement `coalesce_into_sections(nodes) -> Vec<Section>`: group consecutive single-child nodes into one section; start a new section at every branch point and at every node immediately after a branch point
  - [x] Replace current `Compartments::from_sorted_nodes` pipeline so the d_lambda rule is applied per-section, not per-SWC-point
- [x] Expose `capacitance` and `axial_resistivity` on `Compartment` directly (moved from `Channel`)
- [ ] Make `d_lambda_rule` public and part of the builder API
- [ ] Verify `children_idxs` / `parent_idxs` connectivity is correct at branch points after d_lambda subdivision
- [/] Unit tests:
  - [x] Test `coalesce_into_sections` on a known morphology — check section count and total lengths
  - [ ] Verify branch-point connectivity after `d_lambda_rule` on a morphology with actual branches

---

## Synapses (`src/synapses.rs`) — new file

- [ ] Create `src/synapses.rs` and add to `lib.rs`
- [ ] Define `SynapseType` enum: `Exp2Syn`, `Unspecified`
- [ ] Define `Exp2Syn` struct with parameters:
  - [ ] `tau1: f64` (rise time constant)
  - [ ] `tau2: f64` (decay time constant)
  - [ ] `e: f64` (reversal potential)
  - [ ] `i: f64` (current state)
- [ ] Define `SynapseDynamics` trait:
  - [ ] `receive(&mut self, weight: f64)` — apply a synaptic event (increment conductance)
  - [ ] `step(&mut self, dt: f64, v_m: f64) -> f64` — return synaptic current for this timestep
- [ ] Implement `SynapseDynamics` for `Exp2Syn`:
  - [ ] Bi-exponential conductance waveform: `g(t) = weight * (exp(-t/tau2) - exp(-t/tau1))`
  - [ ] Compute `I_syn = g(t) * (v_m - e)`
- [ ] Unit tests:
  - [ ] Test that a single event produces a conductance peak at the correct time
  - [ ] Test that `I_syn` is zero when `v_m == e`

---

## Applying Currents (`src/compartments.rs` or `src/stimuli.rs`)

- [ ] Define a `Stimulus` type (e.g., a step current: `amplitude`, `t_start`, `t_stop`)
- [/] Implement `attach_stimuli` on `Compartments` — assign a stimulus to a specific compartment by index
- [ ] Validate stimulus length matches `T / dt` at simulation start

---

## Solver / Simulation Loop (`src/compartments.rs` or `src/solver.rs`)

Ties channels, dynamics, synapses, and stimuli together. The naive forward-Euler loop is numerically stiff and requires a very small `dt` (~0.025 ms) to stay stable. The target approach is **Crank-Nicolson + Hines algorithm**:

- **Crank-Nicolson** (implicit, 2nd-order) allows ~10× larger `dt` without instability
- **Hines algorithm** solves the resulting tridiagonal system in O(N) by exploiting the tree topology (backward sweep from leaves → root, then forward sweep back down) — this is what NEURON uses internally
- Reference: [Hines 1984](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC1374764/), NEURON Book ch. 4

- [ ] Start with **forward Euler** as a correct-but-slow baseline (easier to validate)
- [/] Implement `Compartments::simulate(dt: f64, T: f64)`:
  - [ ] Allocate voltage trace output: `Vec<Vec<f64>>` (one per compartment)
  - [ ] For each timestep `t`:
    - [ ] Apply external stimulus current to designated compartment(s)
    - [ ] Apply synaptic currents from active synapses
    - [ ] Call `Dynamics::step` on each compartment
    - [ ] Record `membrane_potential()` for each compartment
  - [ ] Return voltage traces
- [ ] Upgrade to **Crank-Nicolson + Hines solver**:
  - [ ] Build the banded tridiagonal matrix from compartment connectivity each timestep
  - [ ] Implement Hines back-substitution (leaf-to-root, then root-to-leaf) over the tree
  - [ ] Validate that results match the Euler baseline on a single-compartment model
- [ ] (Optional) Parallelize independent branches with `rayon`
- [ ] Unit / integration tests:
  - [ ] Single-compartment HH neuron fires at expected rate under constant current
  - [ ] Passive neuron reaches steady-state with correct time constant `τ = R_m * C_m`
  - [ ] Multi-compartment Euler and Hines results match to within tolerance

---

## Python Bindings (`src/lib.rs` + `pyo3`)

- [x] Add `pyo3` as a dependency
- [ ] Expose `Compartments::from_swc(path: &str)` as a Python-callable constructor
- [ ] Expose `Compartments::simulate` result as a `numpy`-compatible array (via `numpy` crate)
- [ ] Expose `HodgkinHuxley`, `Passive` as Python-selectable dynamics options
- [ ] Write a minimal Python smoke test (no pytest needed — a runnable `.py` script is fine)
