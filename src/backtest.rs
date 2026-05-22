//! 回测引擎：按 K 收盘推进，调用任意实现 [`crate::strategy::Strategy`] 的策略，
//! 并按收盘价执行「满仓买 / 全部卖」的简单撮合（仅多头）。
//!
//! 策略与撮合的职责边界：策略只产出 [`crate::strategy::StrategyDirective`]；
//! 本模块负责现金、手续费、持仓与记账。

use chrono::{DateTime, Utc};

use crate::ohlcv::OHLCV;
use crate::strategy::{BarCloseContext, PortfolioSnapshot, Strategy, StrategyDirective};

#[derive(Clone, Copy)]
pub struct BacktestConfig {
    pub initial_cash: f64,
    pub commission_rate: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_cash: 100_000.0,
            commission_rate: 0.0005,
        }
    }
}

pub struct TradeRecord {
    pub bar_index: usize,
    pub timestamp: DateTime<Utc>,
    pub side: &'static str,
    pub qty: f64,
    pub price: f64,
    pub cash_after: f64,
    pub equity_after: f64,
}

pub struct BacktestResult {
    pub strategy_name: String,
    pub config: BacktestConfig,
    pub trades: Vec<TradeRecord>,
    pub final_cash: f64,
    pub final_qty: f64,
    pub last_close: f64,
}

impl BacktestResult {
    pub fn final_equity(&self) -> f64 {
        mark_to_market(self.final_cash, self.final_qty, self.last_close)
    }

    pub fn print_report(&self) {
        println!("\n—— 回测：{} ——", self.strategy_name);
        println!(
            "初始资金 {:.2}，单边手续费 {:.4}%",
            self.config.initial_cash,
            self.config.commission_rate * 100.0,
        );

        if self.trades.is_empty() {
            println!("无成交（样本太短或策略从未发出有效指令）。");
        } else {
            println!("\n成交明细：");
            for trade in &self.trades {
                println!(
                    "  [{:>3}] {} {:>4} @ {:.4}  qty {:.6}  现金 {:.2}  净值 {:.2}",
                    trade.bar_index,
                    trade.timestamp.format("%m-%d %H:%M"),
                    trade.side,
                    trade.price,
                    trade.qty,
                    trade.cash_after,
                    trade.equity_after,
                );
            }
        }

        let equity = self.final_equity();
        let total_return_pct =
            (equity - self.config.initial_cash) / self.config.initial_cash * 100.0;
        println!(
            "\n期末：现金 {:.2} + 持仓 {:.6} × 收盘价 {:.4} = 净值 {:.2}｜总收益率 {:.2}%",
            self.final_cash, self.final_qty, self.last_close, equity, total_return_pct,
        );
    }
}

const POSITION_NEAR_ZERO: f64 = f64::EPSILON;

#[derive(Clone, Copy)]
struct Portfolio {
    cash: f64,
    qty: f64,
}

impl Portfolio {
    fn snapshot(&self) -> PortfolioSnapshot {
        PortfolioSnapshot {
            cash: self.cash,
            qty: self.qty,
        }
    }

    fn can_buy_all(&self) -> bool {
        self.qty.abs() < POSITION_NEAR_ZERO && self.cash > 0.0
    }

    fn can_sell_all(&self) -> bool {
        self.qty > POSITION_NEAR_ZERO
    }
}

fn mark_to_market(cash: f64, qty: f64, mark: f64) -> f64 {
    cash + qty * mark
}

fn record_trade(
    trades: &mut Vec<TradeRecord>,
    bar_index: usize,
    bar: &OHLCV,
    side: &'static str,
    qty: f64,
    portfolio: &Portfolio,
) {
    let price = bar.close;
    trades.push(TradeRecord {
        bar_index,
        timestamp: bar.timestamp,
        side,
        qty,
        price,
        cash_after: portfolio.cash,
        equity_after: mark_to_market(portfolio.cash, portfolio.qty, price),
    });
}

fn buy_all_at_close(
    portfolio: &mut Portfolio,
    trades: &mut Vec<TradeRecord>,
    bar_index: usize,
    bar: &OHLCV,
    fee_rate: f64,
) {
    let price = bar.close;
    let buy_qty = portfolio.cash / (price * (1.0 + fee_rate));
    let spend = buy_qty * price * (1.0 + fee_rate);
    portfolio.cash -= spend;
    portfolio.qty = buy_qty;
    record_trade(trades, bar_index, bar, "买入", buy_qty, portfolio);
}

fn sell_all_at_close(
    portfolio: &mut Portfolio,
    trades: &mut Vec<TradeRecord>,
    bar_index: usize,
    bar: &OHLCV,
    fee_rate: f64,
) {
    let price = bar.close;
    let sell_qty = portfolio.qty;
    let gross = sell_qty * price;
    portfolio.cash += gross - gross * fee_rate;
    portfolio.qty = 0.0;
    record_trade(trades, bar_index, bar, "卖出", sell_qty, portfolio);
}

/// 对已收盘 OHLC 序列跑一次回测；策略在每一步收盘后下达 [`StrategyDirective`]。
pub fn run_backtest<S: Strategy>(
    bars: &[OHLCV],
    strategy: &mut S,
    config: BacktestConfig,
) -> BacktestResult {
    let strategy_name = strategy.name();

    let close_prices: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
    let last_close = close_prices.last().copied().unwrap_or(0.0);

    let mut portfolio = Portfolio {
        cash: config.initial_cash,
        qty: 0.0,
    };
    let mut trades = Vec::new();

    let first_bar_index = strategy.first_live_bar_index();
    if bars.len() <= first_bar_index {
        return BacktestResult {
            strategy_name,
            config,
            trades,
            final_cash: portfolio.cash,
            final_qty: portfolio.qty,
            last_close,
        };
    }

    let fee_rate = config.commission_rate;

    for bar_index in first_bar_index..bars.len() {
        let ctx = BarCloseContext {
            bar_index,
            bars,
            close_prices: close_prices.as_slice(),
            portfolio: portfolio.snapshot(),
        };

        let directive = strategy.on_bar_close(ctx);
        let bar = &bars[bar_index];

        match directive {
            StrategyDirective::EnterLongSpendAllCash if portfolio.can_buy_all() => {
                buy_all_at_close(&mut portfolio, &mut trades, bar_index, bar, fee_rate);
            }
            StrategyDirective::ExitLong if portfolio.can_sell_all() => {
                sell_all_at_close(&mut portfolio, &mut trades, bar_index, bar, fee_rate);
            }
            _ => {}
        }
    }

    BacktestResult {
        strategy_name,
        config,
        trades,
        final_cash: portfolio.cash,
        final_qty: portfolio.qty,
        last_close,
    }
}
