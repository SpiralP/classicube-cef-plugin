use std::time::Duration;

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds - hours * 3600) / 60;
    let seconds = seconds - hours * 3600 - minutes * 60;

    if hours != 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
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
