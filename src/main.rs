//! 读取 OHLCV CSV：`timestamp,open,high,low,close,volume`（RFC3339 时间戳），并在终端画 ASCII K 线。

use std::error::Error;
use std::fs::File;

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
struct OHLCV {
    timestamp: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: u64,
}

fn read_ohlcv_csv(rdr: impl std::io::Read) -> Result<Vec<OHLCV>, Box<dyn Error>> {
    Ok(csv::Reader::from_reader(rdr)
        .deserialize::<OHLCV>()
        .collect::<Result<Vec<_>, _>>()?)
}

/// 价格 → K 线图行号（从上到下：高价在上，`chart_height` 向下价格降低）。
fn price_2_chart_row(
    price: f64,
    lowest_low_price: f64,
    highest_high_price: f64,
    chart_height: usize,
) -> usize {
    if chart_height <= 1 {
        return 0;
    }
    let span = highest_high_price - lowest_low_price;
    if span.abs() < f64::EPSILON {
        return chart_height / 2;
    }
    let fraction_from_top = (highest_high_price - price) / span;
    (fraction_from_top * (chart_height - 1) as f64)
        .round()
        .clamp(0., (chart_height - 1) as f64) as usize
}

fn print_ascii_candles(bars: &[OHLCV], chart_height: usize, candle_column_stride: usize) {
    const AXIS_LABEL_WIDTH: usize = 8;
    const DETAIL_TAIL_COUNT: usize = 16;

    if bars.is_empty() {
        println!("（无数据）");
        return;
    }

    let mut lowest_low_price = bars[0].low;
    let mut highest_high_price = bars[0].high;
    for bar in bars.iter().skip(1) {
        lowest_low_price = lowest_low_price.min(bar.low);
        highest_high_price = highest_high_price.max(bar.high);
    }
    let price_span = highest_high_price - lowest_low_price;
    if price_span < f64::EPSILON {
        lowest_low_price -= 1.0;
        highest_high_price += 1.0;
    }

    let chart_width = candle_column_stride * bars.len();
    let mut canvas = vec![vec![' '; chart_width]; chart_height];

    for (bar_index, bar) in bars.iter().enumerate() {
        let center_column_index = bar_index * candle_column_stride + candle_column_stride / 2;

        let chart_row_high =
            price_2_chart_row(bar.high, lowest_low_price, highest_high_price, chart_height);
        let chart_row_low =
            price_2_chart_row(bar.low, lowest_low_price, highest_high_price, chart_height);

        let open_close_high = bar.open.max(bar.close);
        let open_close_low = bar.open.min(bar.close);
        let chart_row_body_upper = price_2_chart_row(
            open_close_high,
            lowest_low_price,
            highest_high_price,
            chart_height,
        );
        let chart_row_body_lower = price_2_chart_row(
            open_close_low,
            lowest_low_price,
            highest_high_price,
            chart_height,
        );

        let candle_top_row_index = chart_row_high.min(chart_row_low);
        let candle_bottom_row_index = chart_row_high.max(chart_row_low);
        let body_top_row_index = chart_row_body_upper.min(chart_row_body_lower);
        let body_bottom_row_index = chart_row_body_upper.max(chart_row_body_lower);

        let bullish = bar.close >= bar.open;
        let body_char = if bullish { '█' } else { '▓' };
        // 实体占三列；影线走中间这一列——先画实体，再把整根 K 区间内仍为空白的中间栅格连成竖线。
        for chart_row_index in body_top_row_index..=body_bottom_row_index {
            for column_offset in -1_i32..=1 {
                let column_index = center_column_index as i32 + column_offset;
                if column_index >= 0 {
                    let column_index = column_index as usize;
                    if column_index < chart_width {
                        canvas[chart_row_index][column_index] = body_char;
                    }
                }
            }
        }

        for chart_row_index in candle_top_row_index..=candle_bottom_row_index {
            if canvas[chart_row_index][center_column_index] == ' ' {
                canvas[chart_row_index][center_column_index] = '│';
            }
        }
    }

    let axis_line_prefix_spaces = " ".repeat(AXIS_LABEL_WIDTH + 1);
    let mut previous_axis_decimal_key: Option<i32> = None;
    for chart_row_index in 0..chart_height {
        let axis_tick_price = highest_high_price
            - (chart_row_index as f64 / (chart_height - 1).max(1) as f64)
                * (highest_high_price - lowest_low_price);
        let axis_decimal_key = (axis_tick_price * 10.0).round() as i32;
        let axis_label_text = match previous_axis_decimal_key {
            Some(key) if key == axis_decimal_key => " ".repeat(AXIS_LABEL_WIDTH),
            _ => format!("{:>width$.1}", axis_tick_price, width = AXIS_LABEL_WIDTH),
        };
        previous_axis_decimal_key = Some(axis_decimal_key);
        println!(
            "{}|{}",
            axis_label_text,
            canvas[chart_row_index].iter().collect::<String>(),
        );
    }

    println!("{}{}", axis_line_prefix_spaces, "─".repeat(chart_width));
    println!(
        "{}涨 █ / 跌 ▓ ｜影线 刻度 {:.0}～{:.0}",
        axis_line_prefix_spaces, lowest_low_price, highest_high_price,
    );

    println!();
    println!("序号 时间 (UTC)                         O→C        H-L      V");

    let detail_skip_before = bars.len().saturating_sub(DETAIL_TAIL_COUNT);
    let index_column_width = bars.len().max(1).to_string().len();
    for (row_index_zero_based, bar) in bars.iter().enumerate().skip(detail_skip_before) {
        let bullish = bar.close >= bar.open;
        let open_to_close_display = format!(
            "{}{:.2}→{:.2}",
            if bullish { "+" } else { "" },
            bar.open,
            bar.close
        );
        println!(
            "{:<iw$} {:<30} {:<16} {:.2}-{:.2} {}",
            row_index_zero_based + 1,
            bar.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
            open_to_close_display,
            bar.high,
            bar.low,
            bar.volume,
            iw = index_column_width,
        );
    }
    if detail_skip_before > 0 {
        println!("… 省略前 {} 根明细", detail_skip_before);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let sample = include_str!("../data/sample.csv");
    let bars = match std::env::args().nth(1) {
        Some(path) => read_ohlcv_csv(File::open(path)?)?,
        None => read_ohlcv_csv(sample.as_bytes())?,
    };

    print_ascii_candles(&bars, 14, 5);
    println!("总计 {} 条", bars.len());
    Ok(())
}
