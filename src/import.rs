use super::drivers::RestApi;
use super::storage::Candles;
use chrono::{Duration, NaiveDate};

const STEP: i64 = 500;
pub async fn import(driver: &dyn RestApi, storage: &Candles, exchange: &str, sym: &str, start: &NaiveDate, end: &NaiveDate) -> u64 {
    // TODO: check if candles already exists
    println!("importing candles for {} days", end.signed_duration_since(*start).num_days());
    let mut total: u64 = 0;
    let mut tstamp = start.and_hms(0, 0, 0);
    let end_t = end.and_hms(0, 0, 0);
    while tstamp < end_t {
        let tstamp_end = tstamp + Duration::minutes(STEP);
        let check = storage.check(exchange, sym, &tstamp, &tstamp_end).await;
        if check < STEP as usize {
            println!("found {} candles in storage, expecting {}", check, STEP);
            let candles = driver.get_candles(sym, None, Some(&tstamp), None).await;
            if candles.is_empty() {
                panic!("not getting any candles");
            }
            total += storage.store(exchange, sym, &candles).await.expect("in storing data to DB");
            tstamp = candles.last().expect("last not present").tstamp;
        } else {
            tstamp = tstamp_end;
        }
        println!("{}", tstamp);
    }
    total
}
