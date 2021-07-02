use crate::orders::{Order, Transaction};
use crate::wallets::SpotWallet;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Statistics {
    pub orders: usize,
    pub canceled_orders: usize,
    pub balance_start: f64,
    pub balance: f64,
    pub lowest_balance: f64,
    pub highest_balance: f64,
    pub tx_history: Vec<Transaction>,
    pub trade_win_loss: Vec<f64>,
}

impl Statistics {
    pub fn new(balance_start: f64) -> Self {
        Self {
            orders: 0,
            canceled_orders: 0,
            balance_start,
            balance: balance_start,
            lowest_balance: balance_start,
            highest_balance: balance_start,
            tx_history: Vec::new(),
            trade_win_loss: Vec::new(),
        }
    }

    pub fn report(&self) -> String {
        let wins = self.trade_win_loss.iter().fold((0.0, 0), |(tot, count), trade| {
            if trade.is_sign_positive() {
                return (tot + trade, count + 1);
            }
            (tot, count)
        });
        let losses = self.trade_win_loss.iter().fold((0.0, 0), |(tot, count), trade| {
            if trade.is_sign_negative() {
                return (tot + trade, count + 1);
            }
            (tot, count)
        });
        let avg_win = if wins.1 == 0 { 0.0 } else { wins.0 / wins.1 as f64 };
        let avg_loss = if losses.1 == 0 { 0.0 } else { losses.0 / losses.1 as f64 };
        format!(
            "num orders: {}
                 gain %: {}
                 lowest/highest: {:.3}/{:.3}
                 total transactions: {}
                 total trades : {}
                 wins/losses: {:.3}/{:.3}
                 avg win/loss: {:.3}/{:.3}",
            self.orders,
            (self.balance - self.balance_start) / self.balance_start * 100.0,
            self.lowest_balance, self.highest_balance,
            self.tx_history.len(),
            self.trade_win_loss.len(),
            wins.1,
            losses.1,
            avg_win,
            avg_loss,
        )
    }
    pub fn update_with_last_prices(&mut self, wallet: &SpotWallet, prices: &HashMap<String, f64>) {
        let balance = wallet.assets.iter().fold(0.0, |balance, (sym, price)| {
            balance + prices.get(sym).expect("coin in wallet missing from price list") * price
        });
        self.balance = balance;
        if balance < self.lowest_balance {
            self.lowest_balance = balance;
        }
        if balance > self.highest_balance {
            self.highest_balance = balance
        }
    }
    pub fn update_with_transaction(&mut self, tx: &Transaction) {
        self.tx_history.push(tx.clone());
        if tx.order.tx_ref != 0 {
            let orig_tx = self
                .tx_history
                .iter()
                .find(|past_tx| past_tx.order.id == tx.order.tx_ref)
                .expect("orig_tx");
            let perc = (tx.avg_price - orig_tx.avg_price) / orig_tx.avg_price;
            self.trade_win_loss.push(perc);
            println!("order with ref {} -> {}", tx.order.id, tx.order.tx_ref);
        } else {
            println!("order without ref {}", tx.order.id);
        }
    }
    pub fn update_with_order(&mut self, _ord: &Order) {
        self.orders += 1;
    }
    pub fn update_with_expired_order(&mut self, _ord: &Order) {
        self.canceled_orders += 1;
    }
}
