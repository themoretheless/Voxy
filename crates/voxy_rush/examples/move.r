// Rush position script: move two units per second along local X.
fn advance(position: f64, speed: f64) -> f64 {
    return position + speed * delta
}
[advance(x, 2), y, z]
