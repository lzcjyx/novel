use tauri_app_lib::workflow::pet_window::clamp_pet_position;

#[test]
fn pet_position_is_clamped_back_into_visible_work_area() {
    let clamped = clamp_pet_position(-5000, 9000, 1920, 1080, 220, 96);
    assert_eq!(clamped.x, 0);
    assert_eq!(clamped.y, 984);

    let clamped = clamp_pet_position(3000, -40, 1920, 1080, 220, 96);
    assert_eq!(clamped.x, 1700);
    assert_eq!(clamped.y, 0);
}
