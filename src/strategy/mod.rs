//! 交易策略模块：各文件一种策略，均实现 [`Strategy`]；指标计算见 [`indicators`]。

mod indicators;

pub mod bollinger_mean_reversion;
pub mod donchian_breakout;
pub mod dual_sma_cross;
pub mod rsi_oversold_bounce;

pub use bollinger_mean_reversion::BollingerMeanReversionStrategy;
pub use donchian_breakout::DonchianBreakoutStrategy;
pub use dual_sma_cross::DualSmaCrossStrategy;
pub use rsi_oversold_bounce::RsiOversoldBounceStrategy;

use crate::ohlcv::OHLCV;

/// 撮合账户快照；策略可读现金与持仓。
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct PortfolioSnapshot {
    pub cash: f64,
    pub qty: f64,
}

#[allow(dead_code)]
pub struct BarCloseContext<'a> {
    pub bar_index: usize,
    pub bars: &'a [OHLCV],
    pub close_prices: &'a [f64],
    pub portfolio: PortfolioSnapshot,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StrategyDirective {
    Hold,
    EnterLongSpendAllCash,
    ExitLong,
}

pub trait Strategy {
    fn first_live_bar_index(&self) -> usize;
    fn name(&self) -> String;
    fn on_bar_close(&mut self, ctx: BarCloseContext<'_>) -> StrategyDirective;
}
