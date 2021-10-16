use gdnative::prelude::*;

#[derive(NativeClass)]
#[inherit(Node)]
pub struct HelloWorld;

impl HelloWorld {
    fn new(_owner: &Node) -> Self {
        HelloWorld
    }
}

#[methods]
impl HelloWorld {
    #[export]
    fn _ready(&self, _owner: &Node) {
        godot_print!("Hey, Sphere!");
    }
}

#[derive(NativeClass)]
#[inherit(Spatial)]
pub struct Mover;

impl Mover {
    fn new(_owner: &Spatial) -> Self {
        Mover
    }
}

#[methods]
impl Mover {
    #[export]
    fn _ready(&self, _owner: &Spatial) {
        godot_print!("Move baby move");
    }

    #[export]
    fn _process(&self, owner: &Spatial, delta: f32) {
        Spatial::translate(
            owner,
            Vector3 {
                x: 1. * delta,
                y: 0.,
                z: 0.,
                ..Default::default()
            },
        );
    }
}

fn init(handle: InitHandle) {
    handle.add_class::<HelloWorld>();
    handle.add_class::<Mover>();
}

godot_init!(init);
