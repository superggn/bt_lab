//! =============================================================================
//! 策略：RSI 超卖反弹（振荡器均值回归）
//! =============================================================================
//! 类型：逆势 / 均值回归 — 用 RSI 识别短期超卖，回升至中性区后离场。
//!
//! 买入：上一根 RSI ≤ oversold，当前根 RSI > oversold（脱离超卖区）。
//! 卖出：上一根 RSI ≥ exit_threshold，当前根 RSI < exit_threshold（升破中性线后回落）。
//!
//! 与双均线、布林带不同：信号来自涨跌动能比率，而非价格通道或均线交叉。
//! 默认示例参数：period=14，oversold=30，exit=50。
//! =============================================================================

use super::indicators::rsi_at_bar_index;
use super::{BarCloseContext, Strategy, StrategyDirective};

pub struct RsiOversoldBounceStrategy {
    period: usize,
    oversold_threshold: f64,
    exit_threshold: f64,
}

impl RsiOversoldBounceStrategy {
    pub fn new(period: usize, oversold_threshold: f64, exit_threshold: f64) -> Self {
        assert!(period >= 2);
        assert!(oversold_threshold < exit_threshold);
        Self {
            period,
            oversold_threshold,
            exit_threshold,
        }
    }
}

impl Strategy for RsiOversoldBounceStrategy {
    fn first_live_bar_index(&self) -> usize {
        self.period
    }

    fn name(&self) -> String {
        format!(
            "RSI{} 超卖反弹 {:.0}→{:.0}",
            self.period, self.oversold_threshold, self.exit_threshold
        )
    }

    fn on_bar_close(&mut self, ctx: BarCloseContext<'_>) -> StrategyDirective {
        let bar_index = ctx.bar_index;
        let prev_bar_index = bar_index - 1;

        let Some(previous_rsi) = rsi_at_bar_index(ctx.close_prices, prev_bar_index, self.period)
        else {
            return StrategyDirective::Hold;
        };
        let Some(current_rsi) = rsi_at_bar_index(ctx.close_prices, bar_index, self.period) else {
            return StrategyDirective::Hold;
        };

        if previous_rsi <= self.oversold_threshold && current_rsi > self.oversold_threshold {
            return StrategyDirective::EnterLongSpendAllCash;
        }
        if previous_rsi >= self.exit_threshold && current_rsi < self.exit_threshold {
            return StrategyDirective::ExitLong;
        }

        StrategyDirective::Hold
    }
}
