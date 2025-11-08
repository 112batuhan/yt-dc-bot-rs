use std::{collections::HashSet, env};

use poise::{Framework, FrameworkOptions, PrefixFrameworkOptions};
use serenity::all::GatewayIntents;
use songbird::SerenityInit;

use yt_dc_bot::{
    commands::{self, Data},
    events::DefaultHandler,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let framework_options = FrameworkOptions {
        commands: vec![
            commands::help(),
            commands::register(),
            commands::play(),
            commands::skip(),
            commands::clear(),
        ],
        prefix_options: PrefixFrameworkOptions {
            prefix: Some(".".to_string()),
            ..Default::default()
        },
        owners: HashSet::from([env::var("OWNER_ID")
            .expect("Expected an owner id")
            .parse()
            .unwrap()]),

        ..Default::default()
    };

    let framework = Framework::new(framework_options, |ctx, _ready, _framework| {
        Box::pin(async move {
            let songbird = songbird::get(ctx)
                .await
                .expect("Songbird Voice client not initialized")
                .clone();

            Ok(Data {
                songbird,
                reqwest_client: reqwest::Client::new(),
            })
        })
    });

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::Client::builder(&token, intents)
        .event_handler(DefaultHandler)
        .register_songbird()
        .framework(framework)
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    tracing::info!("Received Ctrl-C, shutting down.");
}
