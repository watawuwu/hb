use std::time::SystemTime;

pub fn now_ts() -> f64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        // No errors occurred
        .unwrap()
        .as_secs_f64()
}
