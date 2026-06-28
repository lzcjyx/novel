#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetPosition {
    pub x: i32,
    pub y: i32,
}

pub fn clamp_pet_position(
    x: i32,
    y: i32,
    work_width: i32,
    work_height: i32,
    window_width: i32,
    window_height: i32,
) -> PetPosition {
    let max_x = (work_width - window_width).max(0);
    let max_y = (work_height - window_height).max(0);
    PetPosition {
        x: x.clamp(0, max_x),
        y: y.clamp(0, max_y),
    }
}
