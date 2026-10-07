mod config;
mod scheduler;
mod store;

use std::sync::Arc;

use chrono::Local;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use config::Config;
use store::{Store, TS_FORMAT};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "привязать бота к этому чату")]
    Start,
    #[command(description = "записи за сегодня")]
    Today,
    #[command(description = "расписание опросов")]
    Schedule,
}

fn help() -> String {
    Command::descriptions().to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
    let cfg = Config::from_env()?;
    let store = Arc::new(Store::open(&cfg.db_path)?);
    let bot = Bot::from_env();
    bot.set_my_commands(Command::bot_commands()).await?;

    info!(db = %cfg.db_path, work_start = cfg.work_start, work_end = cfg.work_end, "bot started");
    tokio::spawn(scheduler::run(bot.clone(), cfg.clone(), store.clone()));

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .branch(dptree::entry().filter_command::<Command>().endpoint(on_command))
                .branch(dptree::endpoint(on_message)),
        )
        .branch(Update::filter_edited_message().endpoint(on_edit));
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![store, Arc::new(cfg)])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

fn is_owner(store: &Store, chat_id: i64, claim: bool) -> bool {
    let owner = if claim {
        store.claim_owner(chat_id).map(Some)
    } else {
        store.owner()
    };
    match owner {
        Ok(Some(id)) => id == chat_id,
        Ok(None) => false,
        Err(e) => {
            error!("failed to read owner: {e}");
            false
        }
    }
}

async fn on_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    store: Arc<Store>,
    cfg: Arc<Config>,
) -> ResponseResult<()> {
    debug!(chat_id = msg.chat.id.0, ?cmd, "command");
    if !is_owner(&store, msg.chat.id.0, matches!(cmd, Command::Start)) {
        return Ok(());
    }
    let reply = match cmd {
        Command::Start => format!("Привязан к этому чату.\n{}", help()),
        Command::Today => today(&store),
        Command::Schedule => scheduler::describe(&cfg),
    };
    bot.send_message(msg.chat.id, reply).await?;
    Ok(())
}

async fn on_message(bot: Bot, msg: Message, store: Arc<Store>) -> ResponseResult<()> {
    let Some(text) = msg.text() else {
        return Ok(());
    };
    debug!(chat_id = msg.chat.id.0, msg_id = msg.id.0, len = text.len(), "message");
    if !is_owner(&store, msg.chat.id.0, false) {
        return Ok(());
    }

    let reply = if text.starts_with('/') {
        help()
    } else {
        match store.add(&Local::now().format(TS_FORMAT).to_string(), text, msg.id.0) {
            Ok(()) => "✓".to_string(),
            Err(e) => format!("Ошибка записи: {e}"),
        }
    };
    bot.send_message(msg.chat.id, reply).await?;
    Ok(())
}

async fn on_edit(bot: Bot, msg: Message, store: Arc<Store>) -> ResponseResult<()> {
    let Some(text) = msg.text() else {
        return Ok(());
    };
    debug!(chat_id = msg.chat.id.0, msg_id = msg.id.0, len = text.len(), "edit");
    if text.starts_with('/') || !is_owner(&store, msg.chat.id.0, false) {
        return Ok(());
    }
    let reply = match store.update(msg.id.0, text) {
        Ok(true) => "✎ обновлено".to_string(),
        Ok(false) => return Ok(()),
        Err(e) => format!("Ошибка записи: {e}"),
    };
    bot.send_message(msg.chat.id, reply).await?;
    Ok(())
}

fn today(store: &Store) -> String {
    let start = Local::now().format("%Y-%m-%d 00:00:00").to_string();
    match store.since(&start) {
        Ok(entries) if entries.is_empty() => "Сегодня записей нет".to_string(),
        Ok(entries) => entries
            .iter()
            .map(|(ts, text)| format!("{} — {text}", &ts[11..16]))
            .collect::<Vec<_>>()
            .join("\n"),
        Err(e) => format!("Ошибка чтения: {e}"),
    }
}
