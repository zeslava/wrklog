use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, TimeDelta, TimeZone, Timelike, Weekday};
use teloxide::prelude::*;
use tracing::{debug, error};

use crate::config::Config;
use crate::store::Store;

pub async fn run(bot: Bot, cfg: Config, store: Arc<Store>) {
    loop {
        let now = Local::now();
        tokio::time::sleep(Duration::from_secs(60 - now.second() as u64)).await;

        if !should_prompt(Local::now(), &cfg) {
            continue;
        }
        match store.owner() {
            Ok(Some(owner)) => {
                debug!(owner, "sending prompt");
                if let Err(e) = bot.send_message(ChatId(owner), "Что сделал?").await {
                    error!("failed to send prompt: {e}");
                }
            }
            Ok(None) => debug!("no owner, skipping prompt"),
            Err(e) => error!("failed to read owner: {e}"),
        }
    }
}

pub fn describe(cfg: &Config) -> String {
    let hours = (cfg.work_start..=cfg.work_end)
        .step_by(cfg.interval_hours as usize)
        .map(|h| format!("{h:02}:00"))
        .collect::<Vec<_>>();
    if hours.is_empty() {
        return "Опросы отключены".to_string();
    }
    let mut text = format!("Пн–Пт: {}", hours.join(", "));
    if let Some(next) = next_prompt(Local::now(), cfg) {
        text += &format!("\nСледующий: {}", next.format("%d.%m %H:%M"));
    }
    text
}

fn next_prompt<Tz: TimeZone>(now: DateTime<Tz>, cfg: &Config) -> Option<DateTime<Tz>> {
    let hour = now.with_minute(0)?.with_second(0)?.with_nanosecond(0)?;
    (1..=24 * 7)
        .map(|i| hour.clone() + TimeDelta::hours(i))
        .find(|t| should_prompt(t.clone(), cfg))
}

fn should_prompt<Tz: TimeZone>(now: DateTime<Tz>, cfg: &Config) -> bool {
    let weekday = !matches!(now.weekday(), Weekday::Sat | Weekday::Sun);
    let hour = now.hour();
    weekday
        && now.minute() == 0
        && (cfg.work_start..=cfg.work_end).contains(&hour)
        && (hour - cfg.work_start).is_multiple_of(cfg.interval_hours)
}
