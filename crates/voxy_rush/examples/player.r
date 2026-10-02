mut distance = 0
mut events = 0
mut contacts = 0
fn start() { set_position(self, [0, 0, 0]) }
fn update(delta) {
    if pressed("spawn") { spawn("crate", [position(self)[0] + 2, 0, 0]) }
}
fn fixed_update(delta) {
    distance = position(self)[0] + input("move") * delta * 4
    set_position(self, [distance, position(self)[1], 0])
}
fn on_event(event) {
    events += 1
    if event.name == "physics.contact" { contacts += 1 }
    if event.name == "scene.reward" { set_scale(self, [2, 2, 2]) }
}
fn on_destroy() { }
