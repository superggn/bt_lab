use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct OHLCV {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}
