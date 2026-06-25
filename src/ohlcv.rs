use std::error::Error;
use std::io::Read;

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct OHLCV {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Binance 现货 K 线 CSV：`open_time,open,high,low,close,volume,...`（`open_time` 为毫秒时间戳）。
#[derive(Deserialize)]
struct BinanceKlineRow {
    open_time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// 演示用 CSV：`timestamp,open,high,low,close,volume`（RFC3339 时间戳）。
#[derive(Deserialize)]
struct DemoKlineRow {
    timestamp: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

fn binance_row_2_ohlcv(row: BinanceKlineRow) -> Result<OHLCV, Box<dyn Error>> {
    let timestamp = DateTime::from_timestamp_millis(row.open_time)
        .ok_or_else(|| format!("无效的 open_time 毫秒时间戳: {}", row.open_time))?;
    Ok(OHLCV {
        timestamp,
        open: row.open,
        high: row.high,
        low: row.low,
        close: row.close,
        volume: row.volume,
    })
}

fn demo_row_2_ohlcv(row: DemoKlineRow) -> OHLCV {
    OHLCV {
        timestamp: row.timestamp,
        open: row.open,
        high: row.high,
        low: row.low,
        close: row.close,
        volume: row.volume,
    }
}

pub fn read_ohlcv_csv(rdr: impl Read) -> Result<Vec<OHLCV>, Box<dyn Error>> {
    let mut reader = csv::Reader::from_reader(rdr);
    let headers = reader.headers()?.clone();
    let is_binance_format = headers.iter().any(|name| name == "open_time");

    if is_binance_format {
        reader
            .deserialize::<BinanceKlineRow>()
            .map(|row_result| {
                row_result
                    .map_err(|e| e.into())
                    .and_then(binance_row_2_ohlcv)
            })
            .collect()
    } else {
        reader
            .deserialize::<DemoKlineRow>()
            .map(|row_result| row_result.map(demo_row_2_ohlcv).map_err(|e| e.into()))
            .collect()
    }
}
