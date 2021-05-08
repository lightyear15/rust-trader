use chrono::{Duration, NaiveDateTime};
use rand::Rng;

pub fn generate_random_tstamp(start: &NaiveDateTime, end: &NaiveDateTime) -> NaiveDateTime {
    let time_frame = end.signed_duration_since(*start);
    let mut rng = rand::thread_rng();
    let rand_time = Duration::seconds(rng.gen_range(0..time_frame.num_seconds()));
    *start + rand_time
}
