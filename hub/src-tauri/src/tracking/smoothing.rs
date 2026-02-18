use std::time::Instant;

pub struct InertiaFilter {
    value: f32,
    attack: f32, // Speed to move AWAY from neutral (0.0 to 1.0)
    decay: f32,  // Speed to RETURN to neutral (0.0 to 1.0)
    last_time: Instant,
}

impl InertiaFilter {
    /// Create a new Inertia Filter
    /// attack: 0.0 (freeze) to 1.0 (instant). Speed when value increases (or moves away from 0?)
    /// decay: 0.0 (freeze) to 1.0 (instant). Speed when value decreases (or returns to 0?)
    /// 
    /// Usually for expressions:
    /// - Fast Attack (Open Mouth) -> 0.8
    /// - Slow Decay (Close Mouth) -> 0.1
    pub fn new(attack: f32, decay: f32) -> Self {
        Self {
            value: 0.0,
            attack,
            decay,
            last_time: Instant::now(),
        }
    }

    /// Filter the input value
    /// Assumes input is normalized 0.0 to 1.0 usually.
    pub fn filter(&mut self, target: f32) -> f32 {
        let now = Instant::now();
        let raw_dt = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        
        // Clamp minimum dt to 1ms to prevent freezing at very high iteration rates
        let dt = raw_dt.max(0.001);
        
        let speed = if target > self.value {
            self.attack
        } else {
            self.decay
        };
        
        let frame_scale = dt / 0.01666; // 1.0 at 60fps
        let adjusted_speed = (speed * frame_scale).clamp(0.0, 1.0);
        
        self.value = self.value + (target - self.value) * adjusted_speed;
        
        self.value
    }
}
