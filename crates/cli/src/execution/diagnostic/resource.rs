use serde_json::{Value, json};

pub(super) fn read() -> Value {
    #[cfg(target_os = "linux")]
    {
        linux()
    }
    #[cfg(not(target_os = "linux"))]
    {
        json!({"status":"unavailable","reason":"platform has no configured resource collector"})
    }
}

#[cfg(target_os = "linux")]
fn linux() -> Value {
    let ticks = std::fs::read_to_string("/proc/self/stat")
        .ok()
        .and_then(|text| {
            let (_, fields) = text.rsplit_once(") ")?;
            let fields = fields.split_whitespace().collect::<Vec<_>>();
            Some(json!({
                "user":fields.get(11)?.parse::<u64>().ok()?,
                "system":fields.get(12)?.parse::<u64>().ok()?,
                "waited_children_user":fields.get(13)?.parse::<u64>().ok()?,
                "waited_children_system":fields.get(14)?.parse::<u64>().ok()?
            }))
        });
    let peak = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("VmHWM:")?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()
            })
        });
    json!({
        "scope":"process and reaped children; not entire descendant tree",
        "cpu_unit":"kernel_clock_ticks",
        "cpu":ticks,
        "process_peak_rss_kib":peak,
        "child_peak_rss":null,
        "io_wait":null,
        "cgroup":{"status":"unavailable","reason":"exclusive workload scope not established"}
    })
}
