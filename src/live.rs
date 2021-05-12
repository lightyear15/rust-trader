use crate::drivers::{LiveFeed, RestApi};
use crate::strategies::SpotSinglePairStrategy;
use std::thread::sleep;

pub async fn run_live(mut rest: Box<dyn RestApi>, mut feed: Box<dyn LiveFeed>, mut strategy: Box<dyn SpotSinglePairStrategy>) {
    let sleep_t = chrono::Duration::seconds(10).to_std().unwrap();
    println!("starting strategy {}", strategy.name());
    sleep(sleep_t);
    loop {
        let msg = feed.next().await;
        println!("{:?}", msg);
    }
}
