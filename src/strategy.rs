//! 交易策略：实现对 [`Strategy`] 的具体逻辑；与撮合 / 账务无关。

use crate::ohlcv::OHLCV;

// --------------------------------------------------------------------------- 策略可见的上下文

/// 撮合账户快照；示例策略可以不用，自定义策略可读。
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct PortfolioSnapshot {
    pub cash: f64,
    /// 多头数量（空仓为 0）。
    pub qty: f64,
}

#[allow(dead_code)]
pub struct BarCloseContext<'a> {
    /// 当前这根 K 已完成收盘：`bars[bar_index]` 可取且含有效 `close`。
    pub bar_index: usize,
    pub bars: &'a [OHLCV],
    pub close_prices: &'a [f64],
    pub portfolio: PortfolioSnapshot,
}

// --------------------------------------------------------------------------- 策略输出 → 撮合层解释

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StrategyDirective {
    /// 本根不产生委托。
    Hold,
    /// 用全部可用现金在该根收盘价建立多头（若已持仓或无现金则由撮合忽略）。
    EnterLongSpendAllCash,
    /// 平掉当前全部多头（若已空仓则由撮合忽略）。
    ExitLong,
}

// --------------------------------------------------------------------------- 通用策略接口

pub trait Strategy {
    /// 从该索引起才调用 [`Strategy::on_bar_close`]；更小索引不传（用于均线预热期等）。
    fn first_live_bar_index(&self) -> usize;

    fn name(&self) -> String;

    fn on_bar_close(&mut self, ctx: BarCloseContext<'_>) -> StrategyDirective;
}

// --------------------------------------------------------------------------- 双均线（收盘价 SMA）金叉 / 死叉 — 趋势跟踪

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

        let Some(previous_ribbon) = SmaRibbon::read(
            ctx.close_prices,
            prev_bar_index,
            self.fast_period,
            self.slow_period,
        ) else {
            return StrategyDirective::Hold;
        };
        let Some(current_ribbon) = SmaRibbon::read(
            ctx.close_prices,
            bar_index,
            self.fast_period,
            self.slow_period,
        ) else {
            return StrategyDirective::Hold;
        };

        match crossing_kind(previous_ribbon, current_ribbon) {
            RibbonCrossKind::Golden => StrategyDirective::EnterLongSpendAllCash,
            RibbonCrossKind::Death => StrategyDirective::ExitLong,
            RibbonCrossKind::None => StrategyDirective::Hold,
        }
    }
}

// --------------------------------------------------------------------------- 布林带均值回归 — 波动率通道（与均线交叉截然不同）

/// 布林带（Bollinger Bands）均值回归：价格跌入下轨后收回带内买入，升破中轨后跌回带内卖出。
///
/// - **入场**：上一根收盘 ≤ 下轨，当前根收盘 > 下轨（自下向上突破下轨，视为超卖反弹）。
/// - **出场**：`exit_at_upper_band = true` 时升破上轨止盈；`false` 时涨回中轨后回落卖出。
///
/// 与双均线策略对比：不比较两条均线的交叉，而是用 **标准差定义的波动通道** 判断偏离与回归。
pub struct BollingerMeanReversionStrategy {
    period: usize,
    std_multiplier: f64,
    /// `true`：涨至上轨后回落卖出；`false`：涨至中轨后回落卖出。
    exit_at_upper_band: bool,
}

impl BollingerMeanReversionStrategy {
    pub fn new(period: usize, std_multiplier: f64) -> Self {
        Self::with_exit_band(period, std_multiplier, false)
    }

    pub fn with_exit_band(period: usize, std_multiplier: f64, exit_at_upper_band: bool) -> Self {
        assert!(period >= 2, "BollingerMeanReversionStrategy: period ≥ 2");
        assert!(
            std_multiplier > 0.0,
            "BollingerMeanReversionStrategy: std_multiplier > 0",
        );
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

        // 收盘自下向上突破下轨 → 买入。
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

// --------------------------------------------------------------------------- 共享指标

#[derive(Clone, Copy)]
struct SmaRibbon {
    fast: f64,
    slow: f64,
}

impl SmaRibbon {
    fn read(
        closes: &[f64],
        bar_index: usize,
        fast_period: usize,
        slow_period: usize,
    ) -> Option<Self> {
        Some(Self {
            fast: sma_tail_mean(closes, bar_index, fast_period)?,
            slow: sma_tail_mean(closes, bar_index, slow_period)?,
        })
    }
}

#[derive(Clone, Copy)]
struct BollingerBands {
    middle: f64,
    lower: f64,
    upper: f64,
}

fn bollinger_bands_at_bar_index(
    closes: &[f64],
    bar_index: usize,
    period: usize,
    std_multiplier: f64,
) -> Option<BollingerBands> {
    let middle = sma_tail_mean(closes, bar_index, period)?;
    let start_bar_index = bar_index + 1 - period;
    let variance = closes[start_bar_index..=bar_index]
        .iter()
        .map(|close| {
            let delta = close - middle;
            delta * delta
        })
        .sum::<f64>()
        / period as f64;
    let std_dev = variance.sqrt();
    let band_width = std_multiplier * std_dev;
    Some(BollingerBands {
        middle,
        lower: middle - band_width,
        upper: middle + band_width,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RibbonCrossKind {
    None,
    Golden,
    Death,
}

fn crossing_kind(previous: SmaRibbon, current: SmaRibbon) -> RibbonCrossKind {
    let prev_fast_at_or_below_slow = previous.fast <= previous.slow;
    let prev_fast_at_or_above_slow = previous.fast >= previous.slow;
    let curr_fast_above_slow = current.fast > current.slow;
    let curr_fast_below_slow = current.fast < current.slow;

    if prev_fast_at_or_below_slow && curr_fast_above_slow {
        RibbonCrossKind::Golden
    } else if prev_fast_at_or_above_slow && curr_fast_below_slow {
        RibbonCrossKind::Death
    } else {
        RibbonCrossKind::None
    }
}

fn sma_tail_mean(closes: &[f64], end_bar_index: usize, period: usize) -> Option<f64> {
    if period == 0 || end_bar_index + 1 < period {
        return None;
    }
    let start_bar_index = end_bar_index + 1 - period;
    let sum: f64 = closes[start_bar_index..=end_bar_index].iter().sum();
    Some(sum / period as f64)
}
