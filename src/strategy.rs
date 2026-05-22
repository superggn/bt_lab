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

// --------------------------------------------------------------------------- 双均线（收盘价 SMA）金叉 / 死叉

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
