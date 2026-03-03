#[derive(Copy, Clone, Default)]
pub enum ChannelType {
    #[default]
    Unspecified,
    Sodium(Sodium),
    Potassium(Potassium),
    Leak(Leak),
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Channel {
    pub resistance: f64,
    pub capacitance: f64,
    pub conductance: f64,
    pub axial_resistivity: f64,
}

#[derive(Copy, Clone, Default)]
pub struct Sodium {
    pub channel: Channel,
}
#[derive(Copy, Clone, Default)]
pub struct Potassium {
    pub channel: Channel,
}
#[derive(Copy, Clone, Default)]
pub struct Leak {
    pub channel: Channel,
}

pub trait ChannelDynamics {
    // Conductance of the voltage-gated channel
    fn conductance(voltage: f64) {}

    // the rate constants for each gating variable
    fn rate_constant(voltage: f64) {}
}

impl ChannelDynamics for Sodium {}
impl ChannelDynamics for Potassium {}
impl ChannelDynamics for Leak {}
