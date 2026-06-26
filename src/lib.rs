pub mod ascii_kline;
pub mod backtest;
pub mod ohlcv;
pub mod strategy;

#[cfg(test)]
mod strategy_suite {
    use crate::backtest::{BacktestConfig, run_backtest};
    use crate::ohlcv::read_ohlcv_csv;
    use crate::strategy::{
        BollingerMeanReversionStrategy, DonchianBreakoutStrategy, DualSmaCrossStrategy,
        RsiOversoldBounceStrategy, Strategy,
    };

    fn return_pct(cfg: BacktestConfig, equity: f64) -> f64 {
        (equity - cfg.initial_cash) / cfg.initial_cash * 100.0
    }

    fn run_one<S: Strategy>(
        bars: &[crate::ohlcv::OHLCV],
        cfg: BacktestConfig,
        mut strategy: S,
    ) -> (String, f64, usize) {
        let name = strategy.name();
        let result = run_backtest(bars, &mut strategy, cfg);
        (
            name,
            return_pct(cfg, result.final_equity()),
            result.trades.len(),
        )
    }

    #[test]
    fn all_strategies_on_eth_may() {
        let bars = read_ohlcv_csv(include_str!("../data/ETHUSDT-1h-2026-05.csv").as_bytes())
            .expect("read csv");
        let cfg = BacktestConfig::default();

        let cases: Vec<(String, f64, usize)> = vec![
            run_one(&bars, cfg, DualSmaCrossStrategy::new(2, 5)),
            run_one(
                &bars,
                cfg,
                BollingerMeanReversionStrategy::with_exit_band(7, 2.4, true),
            ),
            run_one(&bars, cfg, RsiOversoldBounceStrategy::new(14, 30.0, 50.0)),
            run_one(&bars, cfg, DonchianBreakoutStrategy::new(48)),
        ];

        for (name, ret, trades) in cases {
            println!("{name}: {ret:+.2}% ({trades} 笔)");
        }

        let buy_hold = if bars.len() >= 2 {
            (bars.last().unwrap().close - bars[0].close) / bars[0].close * 100.0
        } else {
            0.0
        };
        println!("买入持有: {buy_hold:+.2}%");
    }
}
