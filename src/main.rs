//! 读取 OHLCV CSV：`timestamp,open,high,low,close,volume`（RFC3339 时间戳），ASCII K 线与极简回测小样。

mod ascii_kline;
mod backtest;
mod ohlcv;
mod strategy;

use std::error::Error;
use std::fs::File;

use backtest::{BacktestConfig, run_backtest};
use ohlcv::OHLCV;
use strategy::DualSmaCrossStrategy;

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

    // ascii_kline::print_ascii_candles(&bars, 14, 5);

    let mut strategy = DualSmaCrossStrategy::new(2, 5);
    let backtest_result = run_backtest(&bars, &mut strategy, BacktestConfig::default());
    backtest_result.print_report();

    println!("\n总计 {} 根 K 线", bars.len());
    Ok(())
}
