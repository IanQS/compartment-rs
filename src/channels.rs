use std::ops::Deref;

#[derive(Copy, Clone, Default)]
pub enum ChannelType {
    #[default]
    Unspecified,
    Potassium(Potassium),
    Sodium(Sodium),
    Leak(Leak),
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Channel {
    pub g_bar: f64,
    pub e_rev: f64,
}

#[derive(Copy, Clone, Default)]
pub struct Potassium {
    pub channel: Channel,
    /// Activation gate probability (0..1). Raised to the 4th power in conductance.
    pub n: f64,
}
#[derive(Copy, Clone, Default)]
pub struct Sodium {
    pub channel: Channel,
    /// Activation gate probability (0..1). Raised to the 3rd power in conductance.
    pub m: f64,
    /// Inactivation gate probability (0..1). Multiplied directly into conductance.
    pub h: f64,
}
#[derive(Copy, Clone, Default)]
pub struct Leak {
    pub channel: Channel,
}

impl Deref for Potassium {
    type Target = Channel;

    fn deref(&self) -> &Self::Target {
        &self.channel
    }
}
impl Deref for Sodium {
    type Target = Channel;

    fn deref(&self) -> &Self::Target {
        &self.channel
    }
}
impl Deref for Leak {
    type Target = Channel;

    fn deref(&self) -> &Self::Target {
        &self.channel
    }
}

impl Potassium {
    fn conductance(self, voltage: f64) -> f64 {
        self.g_bar * self.n.powf(4.0) * (voltage - self.e_rev)
    }

    pub(crate) fn alpha_n(voltage: f64) -> f64 {
        let common = voltage + 55.0;
        0.01 * common / (1.0 - (-0.1 * common).exp())
    }

    pub(crate) fn beta_n(voltage: f64) -> f64 {
        0.125 * (-0.0125 * (voltage + 65.0)).exp()
    }

    pub fn gating_variable_n(voltage: f64, m: f64) -> f64 {
        let alpha_n = Potassium::alpha_n(voltage) * (1.0 - m);
        let beta_n = Potassium::beta_n(voltage) * m;
        return alpha_n - beta_n;
    }
}

impl Sodium {
    pub fn conductance(self, voltage: f64) -> f64 {
        self.g_bar * self.m.powf(3.0) * self.h * (voltage - self.e_rev)
    }

    pub(crate) fn alpha_m(voltage: f64) -> f64 {
        let common = voltage + 40.0;
        0.1 * common / (1.0 - (-0.1 * common).exp())
    }

    pub(crate) fn alpha_h(voltage: f64) -> f64 {
        0.07 * (-0.05 * (voltage + 65.0)).exp()
    }
    pub(crate) fn beta_m(voltage: f64) -> f64 {
        4.0 * (-0.0556 * (voltage + 65.0)).exp()
    }
    pub(crate) fn beta_h(voltage: f64) -> f64 {
        let denom = 1.0 + (-0.1 * (voltage + 35.0)).exp();
        1.0 / denom
    }
    pub fn gating_variable_m(voltage: f64, m: f64) -> f64 {
        let alpha_m = Sodium::alpha_m(voltage) * (1.0 - m);
        let beta_m = Sodium::beta_m(voltage) * m;
        return alpha_m - beta_m;
    }
    /// Returns dh/dt = alpha_h(V)*(1 - h) - beta_h(V)*h
    pub fn gating_variable_h(voltage: f64, h: f64) -> f64 {
        let alpha_h = Sodium::alpha_h(voltage) * (1.0 - h);
        let beta_h = Sodium::beta_h(voltage) * h;
        return alpha_h - beta_h;
    }
}

/// Only has conductance - no gating variables that open or close
impl Leak {
    fn conductance(self, voltage: f64) -> f64 {
        self.g_bar * (voltage - self.e_rev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values at V_rest = -68 mV from HH worked example.
    // See: https://neuronaldynamics.epfl.ch (or equivalent HH tutorial).
    //
    // alpha_n=0.049, alpha_m=0.18, alpha_h=0.08
    // beta_n =0.13,  beta_m=4.73,  beta_h=0.036
    //
    // p_inf = alpha / (alpha + beta)
    // n_inf = 0.049 / (0.049 + 0.13) ≈ 0.274
    // m_inf = 0.18  / (0.18  + 4.73) ≈ 0.037
    // h_inf = 0.08  / (0.08  + 0.036)≈ 0.690

    const V_REST: f64 = -68.0;
    const TOL: f64 = 1e-3; // allow small rounding differences vs reference

    #[test]
    fn test_alpha_beta_at_rest() {
        let alpha_n = Potassium::alpha_n(V_REST);
        let beta_n  = Potassium::beta_n(V_REST);
        let alpha_m = Sodium::alpha_m(V_REST);
        let beta_m  = Sodium::beta_m(V_REST);
        let alpha_h = Sodium::alpha_h(V_REST);
        let beta_h  = Sodium::beta_h(V_REST);

        assert!((alpha_n - 0.049).abs() < TOL, "alpha_n={alpha_n:.4}");
        assert!((beta_n  - 0.13 ).abs() < TOL, "beta_n={beta_n:.4}");
        assert!((alpha_m - 0.18 ).abs() < TOL, "alpha_m={alpha_m:.4}");
        assert!((beta_m  - 4.73 ).abs() < TOL, "beta_m={beta_m:.4}");
        assert!((alpha_h - 0.08 ).abs() < TOL, "alpha_h={alpha_h:.4}");
        assert!((beta_h  - 0.036).abs() < TOL, "beta_h={beta_h:.4}");
    }

    #[test]
    fn test_steady_state_gating_at_rest() {
        let alpha_n = Potassium::alpha_n(V_REST);
        let beta_n  = Potassium::beta_n(V_REST);
        let alpha_m = Sodium::alpha_m(V_REST);
        let beta_m  = Sodium::beta_m(V_REST);
        let alpha_h = Sodium::alpha_h(V_REST);
        let beta_h  = Sodium::beta_h(V_REST);

        let n_inf = alpha_n / (alpha_n + beta_n);
        let m_inf = alpha_m / (alpha_m + beta_m);
        let h_inf = alpha_h / (alpha_h + beta_h);

        // At rest, dn/dt = dm/dt = dh/dt = 0, so p_inf is the correct initial condition.
        assert!((n_inf - 0.274).abs() < TOL, "n_inf={n_inf:.4}");
        assert!((m_inf - 0.037).abs() < TOL, "m_inf={m_inf:.4}");
        assert!((h_inf - 0.690).abs() < TOL, "h_inf={h_inf:.4}");
    }
}
