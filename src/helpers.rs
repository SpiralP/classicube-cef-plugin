use std::time::Duration;

use classicube_sys::Vec3;
use nalgebra::Vector3;

pub fn vec3_to_vector3(v: &Vec3) -> Vector3<f32> {
    Vector3::new(v.X, v.Y, v.Z)
}

// fn vector3_to_vec3(v: &Vector3<f32>) -> Vec3 {
//     Vec3::new(v.x, v.y, v.z)
// }

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds - hours * 3600) / 60;
    let seconds = seconds - hours * 3600 - minutes * 60;

    if hours == 0 {
        format!("{minutes:02}:{seconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}

#[test]
fn test_format_duration() {
    for (a, b) in &[
        (Duration::from_secs(2), "00:02"),
        (Duration::from_secs(60), "01:00"),
        (Duration::from_secs(61), "01:01"),
        (Duration::from_secs(60 * 60), "01:00:00"),
        (Duration::from_secs(60 * 60 + 1), "01:00:01"),
        (Duration::from_secs(60 * 60 + 60 + 1), "01:01:01"),
    ] {
        assert_eq!(&format_duration(*a), b);
    }
}
