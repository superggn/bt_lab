//! 策略共用的收盘价 / 高低价指标（窗口均含 `bar_index` 当根已收盘 K）。

pub(crate) fn sma_tail_mean(closes: &[f64], end_bar_index: usize, period: usize) -> Option<f64> {
    if period == 0 || end_bar_index + 1 < period {
        return None;
    }
    let start_bar_index = end_bar_index + 1 - period;
    let sum: f64 = closes[start_bar_index..=end_bar_index].iter().sum();
    Some(sum / period as f64)
}

pub(crate) struct BollingerBands {
    pub middle: f64,
    pub lower: f64,
    pub upper: f64,
}

pub(crate) fn bollinger_bands_at_bar_index(
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
    let band_width = std_multiplier * variance.sqrt();
    Some(BollingerBands {
        middle,
        lower: middle - band_width,
        upper: middle + band_width,
    })
}

pub(crate) struct SmaPair {
    pub fast: f64,
    pub slow: f64,
}

pub(crate) fn sma_pair_at_bar_index(
    closes: &[f64],
    bar_index: usize,
    fast_period: usize,
    slow_period: usize,
) -> Option<SmaPair> {
    Some(SmaPair {
        fast: sma_tail_mean(closes, bar_index, fast_period)?,
        slow: sma_tail_mean(closes, bar_index, slow_period)?,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SmaCrossKind {
    None,
    Golden,
    Death,
}

pub(crate) fn sma_cross_kind(previous: SmaPair, current: SmaPair) -> SmaCrossKind {
    let prev_fast_at_or_below_slow = previous.fast <= previous.slow;
    let prev_fast_at_or_above_slow = previous.fast >= previous.slow;

    if prev_fast_at_or_below_slow && current.fast > current.slow {
        SmaCrossKind::Golden
    } else if prev_fast_at_or_above_slow && current.fast < current.slow {
        SmaCrossKind::Death
    } else {
        SmaCrossKind::None
    }
}

/// Wilder 平滑 RSI。
pub(crate) fn rsi_at_bar_index(closes: &[f64], bar_index: usize, period: usize) -> Option<f64> {
    if period < 2 || bar_index < period {
        return None;
    }

    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period {
        let change = closes[i] - closes[i - 1];
        if change >= 0.0 {
            avg_gain += change;
        } else {
            avg_loss += -change;
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    for i in (period + 1)..=bar_index {
        let change = closes[i] - closes[i - 1];
        avg_gain = (avg_gain * (period - 1) as f64 + change.max(0.0)) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + (-change).max(0.0)) / period as f64;
    }

    if avg_loss < f64::EPSILON {
        return Some(100.0);
    }
    Some(100.0 - (100.0 / (1.0 + avg_gain / avg_loss)))
}

/// `bars[start..=end]` 区间最高价（含当根 high）。
pub(crate) fn highest_high_in_bars(
    bars: &[crate::ohlcv::OHLCV],
    start_bar_index: usize,
    end_bar_index: usize,
) -> f64 {
    bars[start_bar_index..=end_bar_index]
        .iter()
        .map(|bar| bar.high)
        .fold(f64::MIN, f64::max)
}

/// `bars[start..=end]` 区间最低价。
pub(crate) fn lowest_low_in_bars(
    bars: &[crate::ohlcv::OHLCV],
    start_bar_index: usize,
    end_bar_index: usize,
) -> f64 {
    bars[start_bar_index..=end_bar_index]
        .iter()
        .map(|bar| bar.low)
        .fold(f64::MAX, f64::min)
}
