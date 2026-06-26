//! =============================================================================
//! 策略：双均线金叉 / 死叉（趋势跟踪）
//! =============================================================================
//! 类型：趋势跟踪 — 用两条收盘价 SMA 的相对位置判断方向。
//!
//! 买入：上一根快线 ≤ 慢线，当前根快线 > 慢线（金叉）。
//! 卖出：上一根快线 ≥ 慢线，当前根快线 < 慢线（死叉）。
//!
//! 默认示例参数：fast=2，slow=5（短周期，交易频繁）。
//! =============================================================================

use super::indicators::{SmaCrossKind, sma_cross_kind, sma_pair_at_bar_index};
use super::{BarCloseContext, Strategy, StrategyDirective};

pub struct DualSmaCrossStrategy {
    fast_period: usize,
    slow_period: usize,
}

impl DualSmaCrossStrategy {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        assert!(
            fast_period >= 1 && fast_period < slow_period,
            "DualSmaCrossStrategy: 要求 1 ≤ fast_period < slow_period",
        );
        Self {
            fast_period,
            slow_period,
        }
    }
}

impl Strategy for DualSmaCrossStrategy {
    fn first_live_bar_index(&self) -> usize {
        self.slow_period
    }

    fn name(&self) -> String {
        format!("双均线 SMA{} × SMA{}", self.fast_period, self.slow_period)
    }

    fn on_bar_close(&mut self, ctx: BarCloseContext<'_>) -> StrategyDirective {
        let bar_index = ctx.bar_index;
        let prev_bar_index = bar_index - 1;

        let Some(previous) = sma_pair_at_bar_index(
            ctx.close_prices,
            prev_bar_index,
            self.fast_period,
            self.slow_period,
        ) else {
            return StrategyDirective::Hold;
        };
        let Some(current) = sma_pair_at_bar_index(
            ctx.close_prices,
            bar_index,
            self.fast_period,
            self.slow_period,
        ) else {
            return StrategyDirective::Hold;
        };

        match sma_cross_kind(previous, current) {
            SmaCrossKind::Golden => StrategyDirective::EnterLongSpendAllCash,
            SmaCrossKind::Death => StrategyDirective::ExitLong,
            SmaCrossKind::None => StrategyDirective::Hold,
        }
    }
}
