use bevy::math::Vec2;

pub trait GridGenerator {
    fn generate_grid(cell_scale: f32) -> Vec<(Vec2, bool)>;
}

pub struct SquareGridGenerator {
    pub center: Vec2,
    pub dimensions: Vec2,
}

impl GridGenerator for SquareGridGenerator {
    fn generate_grid(_cell_scale: f32) -> Vec<(Vec2, bool)> {
        vec![]
    }
}
