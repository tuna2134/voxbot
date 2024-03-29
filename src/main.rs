use dotenv::dotenv;
use glob::glob;
use serenity::async_trait;
use serenity::client::Context;
use serenity::framework::{
    standard::{
        help_commands,
        macros::{command, group, help},
        Args, CommandGroup, CommandResult, HelpOptions,
    },
    StandardFramework,
};
use serenity::model::{
    channel::Message,
    gateway::{Activity, Ready},
    id::{ChannelId, UserId},
};
use serenity::prelude::GatewayIntents;
use serenity::prelude::*;
use serenity::Client;
use songbird::ffmpeg;
use songbird::input::cached::Memory;
use songbird::SerenityInit;

mod voicevox;
use crate::voicevox::VoiceVox;

use std::collections::HashSet;
use std::env;
use std::fs::{remove_file, File};
use std::io::Write;
use std::sync::Arc;

use tokio::sync::RwLock;

struct DataState {
    voicevox: VoiceVox,
    channels: Vec<ChannelId>,
}

impl TypeMapKey for DataState {
    type Value = Arc<RwLock<DataState>>;
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    for file_path in glob("audio/*.wav").unwrap() {
        match file_path {
            Ok(path) => {
                remove_file(path).unwrap();
            }
            Err(e) => println!("{:?}", e),
        }
    }
    let token = env::var("DISCORD_TOKEN").expect("Invalid token");
    let voicevox_api_url = env::var("VOICEVOX_API_URL").expect("Invalid voicevox api url");
    let intents = GatewayIntents::all();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&GENERAL_GROUP)
        .help(&HELP_COMMAND);
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error");
    {
        let data_state = DataState {
            voicevox: VoiceVox::new(voicevox_api_url),
            channels: Vec::new(),
        };
        let mut data = client.data.write().await;
        data.insert::<DataState>(Arc::new(RwLock::new(data_state)));
    }
    client.start().await.expect("Error");
}

#[command]
#[description = "読み上げを開始します。"]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;
    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);
    let n_channel_id = match channel_id {
        Some(channel_id) => channel_id,
        None => {
            msg.reply(&ctx.http, "チャンネルに参加してください。")
                .await?;
            return Ok(());
        }
    };
    let manager = songbird::get(ctx).await.unwrap().clone();
    let _handler = manager.join(guild_id, n_channel_id).await;
    msg.reply(&ctx.http, "接続しました。").await?;
    let data_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<DataState>().unwrap().clone()
    };
    {
        let mut data_write = data_lock.write().await;
        data_write.channels.push(msg.channel_id);
    }
    Ok(())
}

#[command]
#[description = "読み上げを終了します。"]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;
    let manager = songbird::get(ctx).await.unwrap().clone();
    let call = manager.get(guild_id);
    match call {
        Some(call) => {
            let _handler = call.lock().await.leave().await;
            msg.reply(&ctx.http, "切断しました。").await?;
        }
        None => {
            return Ok(());
        }
    }
    let data_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<DataState>().unwrap().clone()
    };
    {
        let mut data_write = data_lock.write().await;
        data_write.channels.retain(|&x| x != msg.channel_id);
    }
    Ok(())
}

#[help]
#[individual_command_tip = "ヘルプコマンドだよ！"]
#[strikethrough_commands_tip_in_guild = ""]
async fn help_command(
    ctx: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(ctx, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[description("普通のコマンド")]
#[summary("一般")]
#[commands(join, leave)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        ctx.set_activity(Activity::playing("読み上げbot起動中"))
            .await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        let data_read = ctx.data.read().await;
        let raw_data = data_read.get::<DataState>().unwrap();
        let data = raw_data.read().await;
        if !data.channels.contains(&msg.channel_id) {
            return;
        }
        let guild = msg.guild(&ctx.cache).unwrap();
        let guild_id = guild.id;
        let manager = songbird::get(&ctx).await.unwrap();
        let call = manager.get(guild_id);
        match call {
            Some(call) => {
                let audio_query = data.voicevox.get_audio_query(msg.content, 1).await.unwrap();
                let audio = data.voicevox.synthe(1, audio_query).await.unwrap();
                let filename = format!("./audio/{}.wav", msg.id);
                let mut file = File::create(filename.clone()).unwrap();
                file.write_all(&audio).unwrap();
                let tts_src = Memory::new(ffmpeg(filename.clone()).await.unwrap()).unwrap();
                tts_src.raw.spawn_loader();
                let mut call_lock = call.lock().await;
                let _handler = call_lock.play_source(tts_src.try_into().unwrap());
            }
            None => {
                return;
            }
        }
    }
}
