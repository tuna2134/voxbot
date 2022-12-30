use dotenv::dotenv;
use serenity::async_trait;
use serenity::client::Context;
use serenity::framework::{
    standard::{
        macros::{command, group},
        CommandResult,
    },
    StandardFramework,
};
use serenity::model::{channel::Message, gateway::Ready};
use serenity::prelude::GatewayIntents;
use serenity::prelude::*;
use serenity::Client;
use songbird::ffmpeg;
use songbird::input::cached::Memory;
use songbird::SerenityInit;

mod voicevox;
use crate::voicevox::VoiceVox;

use std::env;
use std::fs::{remove_file, File};
use std::io::Write;
use std::sync::Arc;

struct DataState {
    voicevox: VoiceVox,
}

impl TypeMapKey for DataState {
    type Value = Arc<DataState>;
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Invalid token");
    let voicevox_api_url = env::var("VOICEVOX_API_URL").expect("Invalid voicevox api url");
    let intents = GatewayIntents::all();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&GENERAL_GROUP);
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error");
    {
        let data_state = DataState {
            voicevox: VoiceVox::new(voicevox_api_url),
        };
        let mut data = client.data.write().await;
        data.insert::<DataState>(Arc::new(data_state));
    }
    client.start().await.expect("Error");
}

#[command]
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
    Ok(())
}

#[command]
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
    Ok(())
}

#[group]
#[description("汎用コマンド")]
#[summary("一般")]
#[commands(join, leave)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return ();
        }
        let guild = msg.guild(&ctx.cache).unwrap();
        let guild_id = guild.id;
        let manager = songbird::get(&ctx).await.unwrap();
        let call = manager.get(guild_id);
        match call {
            Some(call) => {
                let data_read = ctx.data.read().await;
                let data = data_read.get::<DataState>().unwrap();
                let audio_query = data.voicevox.get_audio_query(msg.content, 1).await.unwrap();
                let audio = data.voicevox.synthe(1, audio_query).await.unwrap();
                let filename = format!("./audio/{}.wav", msg.id);
                let mut file = File::create(filename.clone()).unwrap();
                file.write_all(&audio).unwrap();
                let tts_src = Memory::new(ffmpeg(filename.clone()).await.unwrap()).unwrap();
                remove_file(filename.clone()).unwrap();
                tts_src.raw.spawn_loader();
                let mut call_lock = call.lock().await;
                let _handler = call_lock.play_source(tts_src.try_into().unwrap());
            }
            None => {
                return ();
            }
        }
    }
}
