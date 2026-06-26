//! =============================================================================
//! 策略：布林带均值回归（波动率通道）
//! =============================================================================
//! 类型：均值回归 — 用 SMA ± k·σ 构成通道，赌价格偏离后回归。
//!
//! 买入：收盘自下向上突破下轨（超卖后收回带内）。
//! 卖出：
//!   - exit_at_upper_band = true → 收盘升破上轨止盈；
//!   - false → 涨回中轨后向下跌破中轨离场。
//!
//! 默认示例参数：period=7，std=2.4，上轨止盈（在 ETH 1h 5 月样本上网格较优）。
//! =============================================================================

use super::indicators::bollinger_bands_at_bar_index;
use super::{BarCloseContext, Strategy, StrategyDirective};

pub struct BollingerMeanReversionStrategy {
    period: usize,
    std_multiplier: f64,
    exit_at_upper_band: bool,
}

impl BollingerMeanReversionStrategy {
    pub fn new(period: usize, std_multiplier: f64) -> Self {
        Self::with_exit_band(period, std_multiplier, false)
    }

    pub fn with_exit_band(period: usize, std_multiplier: f64, exit_at_upper_band: bool) -> Self {
        assert!(period >= 2, "BollingerMeanReversionStrategy: period ≥ 2");
        assert!(std_multiplier > 0.0);
        Self {
            period,
            std_multiplier,
            exit_at_upper_band,
        }
    }
}

impl Strategy for BollingerMeanReversionStrategy {
    fn first_live_bar_index(&self) -> usize {
        self.period
    }

    fn name(&self) -> String {
        let exit_label = if self.exit_at_upper_band {
            "上轨止盈"
        } else {
            "中轨止盈"
        };
        format!(
            "布林带均值回归 {} × {:.1}σ {}",
            self.period, self.std_multiplier, exit_label
        )
    }

    fn on_bar_close(&mut self, ctx: BarCloseContext<'_>) -> StrategyDirective {
        let bar_index = ctx.bar_index;
        let prev_bar_index = bar_index - 1;

        let Some(previous_bands) = bollinger_bands_at_bar_index(
            ctx.close_prices,
            prev_bar_index,
            self.period,
            self.std_multiplier,
        ) else {
            return StrategyDirective::Hold;
        };
        let Some(current_bands) = bollinger_bands_at_bar_index(
            ctx.close_prices,
            bar_index,
            self.period,
            self.std_multiplier,
        ) else {
            return StrategyDirective::Hold;
        };

        let prev_close = ctx.close_prices[prev_bar_index];
        let current_close = ctx.close_prices[bar_index];

        if prev_close <= previous_bands.lower && current_close > current_bands.lower {
            return StrategyDirective::EnterLongSpendAllCash;
        }

        if self.exit_at_upper_band {
            if prev_close < previous_bands.upper && current_close >= current_bands.upper {
                return StrategyDirective::ExitLong;
            }
        } else if prev_close >= previous_bands.middle && current_close < current_bands.middle {
            return StrategyDirective::ExitLong;
        }

        StrategyDirective::Hold
    }
}
