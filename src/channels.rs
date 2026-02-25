///
/// The channels defined the dynamics that take place within the compartment
/// Some based on: https://nrn.readthedocs.io/en/9.0.0/tutorials/scripting-neuron-basics.html#Biophysical-mechanisms
///

#[derive(Default)]
pub enum ChannelType {
    #[default]
    Unspecified,
    Passive(Passive),
    Extracellular(Extracellular),
    HodgkinHuxley(HodgkinHuxley),
}

#[derive(Default)]
pub struct Channel {
    channel_type: ChannelType,
    resistance: f64,
    capacitance: f64,
    conductance: f64,
}

pub trait Dynamics {
    fn new() -> Self {}
    fn propagate(self) -> () {}
    fn update(self) -> () {}
}

pub struct HodgkinHuxley {}

impl Dynamics for HodgkinHuxley {}
pub struct Extracellular {}
impl Dynamics for Extracellular {}
pub struct Passive {}
impl Dynamics for Passive {}
