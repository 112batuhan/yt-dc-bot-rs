use std::sync::Arc;

use serenity::all::ChannelId;
use serenity::all::Context;
use serenity::all::GuildId;
use serenity::all::Http;
use serenity::all::Ready;
use serenity::all::VoiceState;
use serenity::async_trait;
use songbird::EventContext;
use songbird::Songbird;

use crate::msg_send_error_log;

pub struct TrackEndHandler {
    pub channel_id: ChannelId,
    pub guild_id: GuildId,
    pub http: Arc<Http>,
    pub songbird: Arc<songbird::Songbird>,
}

#[async_trait]
impl songbird::EventHandler for TrackEndHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<songbird::Event> {
        tracing::info!("Track finished in guild {}", self.guild_id);

        // Optional message
        if let EventContext::Track(track_list) = ctx {
            let _ = self
                .channel_id
                .say(
                    self.http.clone(),
                    &format!("Tracks ended: {}.", track_list.len()),
                )
                .await
                .inspect_err(msg_send_error_log);
        }

        if let Some(call_lock) = self.songbird.get(self.guild_id) {
            let mut call = call_lock.lock().await;

            if call.queue().is_empty() && call.current_channel().is_some() {
                tracing::info!("Queue empty, leaving channel in guild {}", self.guild_id);

                call.queue().stop();
                call.remove_all_global_events();

                if let Err(e) = call.leave().await {
                    tracing::error!("Failed to leave voice channel: {:?}", e);
                }
            }
        }

        None
    }
}

pub fn check_if_channel_empty(
    ctx: &serenity::client::Context,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> bool {
    let someone_there = ctx
        .cache
        .guild(guild_id)
        .unwrap()
        .voice_states
        .iter()
        .filter(|(_, state)| state.user_id != ctx.cache.current_user().id)
        .any(|(_id, state)| state.channel_id == Some(channel_id));
    !someone_there
}
pub struct DefaultHandler;

#[async_trait]
impl serenity::client::EventHandler for DefaultHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        tracing::info!("{} is connected!", ready.user.name);
    }
    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        if let Some(guild_id) = new.guild_id {
            let manager = songbird::get(&ctx)
                .await
                .expect("Songbird Voice client not initialized.")
                .clone();

            let Some(call_lock) = manager.get(guild_id) else {
                return;
            };
            let call = call_lock.lock().await;
            let Some(current_channel) = call.current_channel() else {
                tracing::info!("Bot not in a voice channel, cleaning up for guild {guild_id}");
                drop(call);
                clean_disconnect(&manager, guild_id).await;
                return;
            };
            drop(call);

            if check_if_channel_empty(&ctx, guild_id, current_channel.0.into()) {
                tracing::info!(
                    "Channel {} is empty, cleaning up for guild {}",
                    current_channel,
                    guild_id
                );
                clean_disconnect(&manager, guild_id).await;
            }
        }
    }
}

/// Disconnects safely and resets the queue before removing the call handler
async fn clean_disconnect(manager: &std::sync::Arc<Songbird>, guild_id: GuildId) {
    if let Some(call_lock) = manager.get(guild_id) {
        let mut call = call_lock.lock().await;

        call.queue().stop();
        call.remove_all_global_events();

        if let Err(e) = call.leave().await {
            tracing::warn!(
                "Failed to leave voice channel cleanly for {guild_id}: {:?}",
                e
            );
        }

        drop(call);

        if let Err(e) = manager.remove(guild_id).await {
            tracing::warn!("Failed to remove call handler for {guild_id}: {:?}", e);
        }

        tracing::info!("Cleaned up and disconnected from guild {}", guild_id);
    } else {
        tracing::debug!("No active call handler found for guild {}", guild_id);
    }
}
