pub fn state_key(feed_id: &str, network_name: &str) -> String {
    format!("{}:{}", feed_id, network_name)
}

pub fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let mins = seconds / 60;
        let secs = seconds % 60;
        if secs > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}m", mins)
        }
    } else {
        let hours = seconds / 3600;
        let mins = (seconds % 3600) / 60;
        let secs = seconds % 60;
        if mins > 0 && secs > 0 {
            format!("{}h {}m {}s", hours, mins, secs)
        } else if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else if secs > 0 {
            format!("{}h {}s", hours, secs)
        } else {
            format!("{}h", hours)
        }
    }
}
