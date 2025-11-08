use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use serenity::all::Mentionable;
use songbird::Songbird;
use songbird::input::YoutubeDl;

use crate::events::TrackEndHandler;
use crate::msg_send_error_log;

pub struct Data {
    pub songbird: Arc<Songbird>,
    pub reqwest_client: Client,
}

type PoiseContext<'a> = poise::Context<'a, Data, anyhow::Error>;

#[poise::command(prefix_command, owners_only)]
pub async fn register(ctx: PoiseContext<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: PoiseContext<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<()> {
    let config = poise::builtins::HelpConfiguration {
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn play(
    ctx: PoiseContext<'_>,
    #[description = "URL of the YouTube video"] url: String,
) -> Result<()> {
    ctx.defer().await?;

    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in a guild").await?;
            return Ok(());
        }
    };

    let channel_id = {
        let guild = ctx.cache().guild(guild_id).expect("guild should exist");
        guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|state| state.channel_id)
    };

    let Some(connect_to) = channel_id else {
        ctx.say("You must be in a voice channel to use this command")
            .await?;
        return Ok(());
    };

    let songbird_client = &ctx.data().songbird;

    // If bot already in the channel, just enqueue
    if let Some(call_lock) = songbird_client.get(guild_id) {
        let mut call = call_lock.lock().await;

        if call.current_channel() == Some(connect_to.into()) {
            let src = YoutubeDl::new(ctx.data().reqwest_client.clone(), url);
            call.enqueue_input(src.into()).await;

            let _ = ctx
                .reply(format!(
                    "Added the song to the queue in {}",
                    connect_to.mention()
                ))
                .await
                .inspect_err(|e| tracing::error!("Failed to send message: {:?}", e));

            return Ok(());
        }
    }

    // Otherwise, join the channel and start playback
    match songbird_client.join(guild_id, connect_to).await {
        Ok(call_lock) => {
            let mut call = call_lock.lock().await;

            // Remove old events to avoid duplicates
            call.remove_all_global_events();

            // Attach TrackEndHandler
            call.add_global_event(
                songbird::Event::Track(songbird::TrackEvent::End),
                TrackEndHandler {
                    songbird: songbird_client.clone(),
                    channel_id: ctx.channel_id(),
                    guild_id,
                    http: ctx.serenity_context().http.clone(),
                },
            );

            // Enqueue the first track (will start immediately)
            let src = YoutubeDl::new(ctx.data().reqwest_client.clone(), url);
            call.enqueue_input(src.into()).await;

            let _ = ctx
                .reply(format!(
                    "Joined {} and started playback",
                    connect_to.mention()
                ))
                .await
                .inspect_err(|e| tracing::error!("Failed to send message: {:?}", e));
        }
        Err(e) => {
            tracing::error!("Error joining voice channel: {:?}", e);
            let _ = ctx
                .reply(format!("Failed to join channel: {:?}", e))
                .await
                .inspect_err(|e| tracing::error!("Failed to send message: {:?}", e));
        }
    }

    Ok(())
}
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn skip(ctx: PoiseContext<'_>) -> Result<()> {
    ctx.defer().await?;
    let songbird_client = &ctx.data().songbird;

    if let Some(handler_lock) = songbird_client.get(ctx.guild_id().expect("guild_only_command")) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();

        let _ = ctx
            .reply(format!("Song skipped: {} in queue.", queue.len()))
            .await
            .inspect_err(msg_send_error_log);
    } else {
        let _ = ctx
            .reply("Not in a voice channel to play in")
            .await
            .inspect_err(msg_send_error_log);
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn clear(ctx: PoiseContext<'_>) -> Result<()> {
    ctx.defer().await?;
    let songbird_client = &ctx.data().songbird;

    if let Some(handler_lock) = songbird_client.get(ctx.guild_id().expect("guild_only_command")) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();

        let _ = ctx
            .reply("Queue cleared")
            .await
            .inspect_err(msg_send_error_log);
    } else {
        let _ = ctx
            .reply("Not in a voice channel anyway")
            .await
            .inspect_err(msg_send_error_log);
    }

    Ok(())
}
