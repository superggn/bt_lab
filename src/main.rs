//! 读取 OHLCV CSV，ASCII K 线与极简回测小样。

use std::error::Error;
use std::fs::File;

use bt_lab::backtest::{BacktestConfig, run_backtest};
use bt_lab::ohlcv::read_ohlcv_csv;
use bt_lab::strategy::{BollingerMeanReversionStrategy, DualSmaCrossStrategy};

fn main() -> Result<(), Box<dyn Error>> {
    let default_csv = include_str!("../data/ETHUSDT-1h-2026-05.csv");
    let bars = match std::env::args().nth(1) {
        Some(path) => read_ohlcv_csv(File::open(path)?)?,
        None => read_ohlcv_csv(default_csv.as_bytes())?,
    };

    let config = BacktestConfig::default();

    // 趋势跟踪：快慢 SMA 金叉 / 死叉
    let mut sma_strategy = DualSmaCrossStrategy::new(2, 5);
    run_backtest(&bars, &mut sma_strategy, config).print_report();

    // 均值回归：布林带下轨抄底 → 上轨 / 中轨止盈
    let mut bollinger_strategy = BollingerMeanReversionStrategy::with_exit_band(7, 2.4, true);
    run_backtest(&bars, &mut bollinger_strategy, config).print_report();

    println!("\n总计 {} 根 K 线", bars.len());
    Ok(())
}
