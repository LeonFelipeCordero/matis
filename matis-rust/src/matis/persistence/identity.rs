use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}
pub fn uuid(compact: bool) -> Result<String> {
    let mut data = [0u8; 16];
    getrandom::fill(&mut data)?;
    data[6] = (data[6] & 15) | 64;
    data[8] = (data[8] & 63) | 128;
    let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
    Ok(if compact {
        hex
    } else {
        format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
    })
}
