///
/// The channels defined the dynamics that take place within the compartment
/// Some based on: https://nrn.readthedocs.io/en/9.0.0/tutorials/scripting-neuron-basics.html#Biophysical-mechanisms

#[derive(Default, Clone, Copy)]
pub enum CellularDynamics {
    #[default]
    Unspecified,
    Passive(Passive),
    Extracellular(Extracellular),
    HodgkinHuxley(HodgkinHuxley),
}

pub trait Dynamics {
    ///
    /// Propagate
    fn propagate(&self) -> () {}

    /// Update the membrane potential of the various channels...?
    fn update(&mut self) -> () {}
}

#[derive(Copy, Clone)]
pub struct HodgkinHuxley {}

impl Dynamics for HodgkinHuxley {}

#[derive(Copy, Clone)]
pub struct Extracellular {}

impl Dynamics for Extracellular {}

#[derive(Copy, Clone)]
pub struct Passive {}

impl Dynamics for Passive {}
