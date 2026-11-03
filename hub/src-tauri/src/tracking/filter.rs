use std::time::Instant;

#[derive(Clone, Debug)]
pub struct OneEuroFilter {
    min_cutoff: f32,
    beta: f32,
    d_cutoff: f32,
    x_prev: f32,
    dx_prev: f32,
    t_prev: Option<Instant>,
}

impl OneEuroFilter {
    pub fn new(min_cutoff: f32, beta: f32) -> Self {
        Self {
            min_cutoff,
            beta,
            d_cutoff: 1.0,
            x_prev: 0.0,
            dx_prev: 0.0,
            t_prev: None,
        }
    }

    pub fn filter(&mut self, x: f32) -> f32 {
        // Guard against NaN/inf: a single non-finite sample would otherwise
        // be stored in x_prev and poison every future output forever.
        // Drop the bad sample and hold the last good value instead.
        if !x.is_finite() {
            return self.x_prev;
        }

        let t = Instant::now();

        if self.t_prev.is_none() {
            self.x_prev = x;
            self.dx_prev = 0.0;
            self.t_prev = Some(t);
            return x;
        }

        let t_prev = self.t_prev.unwrap();
        let raw_dt = t.duration_since(t_prev).as_secs_f32();
        self.t_prev = Some(t);
        
        // Clamp minimum dt to 1ms to prevent filter freezing at very high iteration rates
        let dt = raw_dt.max(0.001);

        let alpha_d = self.alpha(self.d_cutoff, dt);
        let dx = (x - self.x_prev) / dt;
        let dx_hat = alpha_d * dx + (1.0 - alpha_d) * self.dx_prev;
        
        let cutoff = self.min_cutoff + self.beta * dx_hat.abs();
        let alpha = self.alpha(cutoff, dt);
        
        let x_hat = alpha * x + (1.0 - alpha) * self.x_prev;
        
        self.x_prev = x_hat;
        self.dx_prev = dx_hat;
        
        x_hat
    }

    fn alpha(&self, cutoff: f32, dt: f32) -> f32 {
        let tau = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        1.0 / (1.0 + tau / dt)
    }
}
