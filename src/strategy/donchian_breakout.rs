//! =============================================================================
//! 策略：唐奇安通道突破（价格结构趋势跟踪）
//! =============================================================================
//! 类型：趋势跟踪 — 用过去 N 根 K 的高低点构成通道，突破通道边界顺势交易。
//!
//! 通道统计 **不含当前根**（仅用已收盘历史），避免 lookahead。
//!
//! 买入：当前收盘 > 过去 channel_period 根的最高价（向上突破）。
//! 卖出：当前收盘 < 过去 channel_period 根的最低价（向下突破）。
//!
//! 与双均线不同：不看均线交叉，只看绝对价格是否突破区间极值。
//! 默认示例参数：channel_period=48（约 2 日 1h K）。
//! =============================================================================

use super::indicators::{highest_high_in_bars, lowest_low_in_bars};
use super::{BarCloseContext, Strategy, StrategyDirective};

pub struct DonchianBreakoutStrategy {
    channel_period: usize,
}

impl DonchianBreakoutStrategy {
    pub fn new(channel_period: usize) -> Self {
        assert!(
            channel_period >= 2,
            "DonchianBreakoutStrategy: channel_period ≥ 2"
        );
        Self { channel_period }
    }
}

impl Strategy for DonchianBreakoutStrategy {
    fn first_live_bar_index(&self) -> usize {
        self.channel_period
    }

    fn name(&self) -> String {
        format!("唐奇安突破 {}", self.channel_period)
    }

    fn on_bar_close(&mut self, ctx: BarCloseContext<'_>) -> StrategyDirective {
        let bar_index = ctx.bar_index;
        let channel_start = bar_index - self.channel_period;
        let channel_end = bar_index - 1;

        let highest_high = highest_high_in_bars(ctx.bars, channel_start, channel_end);
        let lowest_low = lowest_low_in_bars(ctx.bars, channel_start, channel_end);
        let close = ctx.close_prices[bar_index];

        if close > highest_high {
            StrategyDirective::EnterLongSpendAllCash
        } else if close < lowest_low {
            StrategyDirective::ExitLong
        } else {
            StrategyDirective::Hold
        }
    }
}
