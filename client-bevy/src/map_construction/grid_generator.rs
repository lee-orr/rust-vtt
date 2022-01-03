use bevy::math::Vec2;

pub trait GridGenerator {
    fn generateGrid(cell_scale: f32) -> Vec<(Vec2, bool)>;
}


pub struct SquareGridGenerator {
    pub center: Vec2,
    pub dimensions: Vec2
}

impl GridGenerator for SquareGridGenerator {
    fn generateGrid(cell_scale: f32) -> Vec<(Vec2, bool)> {

        vec![]
    }
}