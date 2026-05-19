//! 读取 OHLCV CSV：`timestamp,open,high,low,close,volume`（RFC3339 时间戳），并在终端画 ASCII K 线。

mod ascii_kline;
mod ohlcv;

use std::error::Error;
use std::fs::File;

use ohlcv::OHLCV;

fn read_ohlcv_csv(rdr: impl std::io::Read) -> Result<Vec<OHLCV>, Box<dyn Error>> {
    Ok(csv::Reader::from_reader(rdr)
        .deserialize::<OHLCV>()
        .collect::<Result<Vec<_>, _>>()?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let sample = include_str!("../data/sample.csv");
    let bars = match std::env::args().nth(1) {
        Some(path) => read_ohlcv_csv(File::open(path)?)?,
        None => read_ohlcv_csv(sample.as_bytes())?,
    };
    ascii_kline::print_ascii_candles(&bars, 14, 5);
    println!("总计 {} 条", bars.len());
    Ok(())
}
