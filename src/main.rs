//! 读取 OHLCV CSV，对内置策略逐一回测并打印报告。

use std::error::Error;
use std::fs::File;

use bt_lab::backtest::{BacktestConfig, run_backtest};
use bt_lab::ohlcv::read_ohlcv_csv;
use bt_lab::strategy::{
    BollingerMeanReversionStrategy, DonchianBreakoutStrategy, DualSmaCrossStrategy,
    RsiOversoldBounceStrategy, Strategy,
};

fn run_strategy<S: Strategy>(bars: &[bt_lab::ohlcv::OHLCV], config: BacktestConfig, mut s: S) {
    run_backtest(bars, &mut s, config).print_report();
}

fn main() -> Result<(), Box<dyn Error>> {
    let default_csv = include_str!("../data/ETHUSDT-1h-2026-05.csv");
    let bars = match std::env::args().nth(1) {
        Some(path) => read_ohlcv_csv(File::open(path)?)?,
        None => read_ohlcv_csv(default_csv.as_bytes())?,
    };

    let config = BacktestConfig::default();

    println!(
        "数据 {} 根 K 线，初始资金 {:.0}",
        bars.len(),
        config.initial_cash
    );

    run_strategy(&bars, config, DualSmaCrossStrategy::new(2, 5));
    run_strategy(
        &bars,
        config,
        BollingerMeanReversionStrategy::with_exit_band(7, 2.4, true),
    );
    run_strategy(
        &bars,
        config,
        RsiOversoldBounceStrategy::new(14, 30.0, 50.0),
    );
    run_strategy(&bars, config, DonchianBreakoutStrategy::new(48));

    if bars.len() >= 2 {
        let buy_hold = (bars.last().unwrap().close - bars[0].close) / bars[0].close * 100.0;
        println!("\n买入持有参考: {buy_hold:+.2}%");
    }

    Ok(())
}
