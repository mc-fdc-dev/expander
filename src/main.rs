use dotenvy::dotenv;
use regex::Regex;
use std::{env, sync::Arc};
use tokio::sync::Mutex;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_gateway::{EventTypeFlags, StreamExt};
use twilight_http::Client as HttpClient;
use twilight_model::{
    gateway::{
        payload::outgoing::UpdatePresence,
        presence::{Activity, ActivityType, Status},
    },
    id::{
        marker::{ChannelMarker, MessageMarker, UserMarker},
        Id,
    },
    util::{ImageHash, Timestamp},
};
use twilight_util::{
    builder::embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource},
    snowflake::Snowflake,
};

struct Client {
    http: Arc<HttpClient>,
    cache: Arc<InMemoryCache>,
    re: Arc<Regex>,
    shard: Arc<Mutex<Shard>>,
}

struct MessageData {
    content: String,
    author_id: Id<UserMarker>,
    channel_name: String,
    id: Id<MessageMarker>,
    image: Option<String>,
}

struct Author {
    name: String,
    id: Id<UserMarker>,
    avatar: Option<ImageHash>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Booting expander...");
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN")?;
    let intents = Intents::GUILD_MESSAGES
        | Intents::GUILDS
        | Intents::MESSAGE_CONTENT
        | Intents::GUILD_MEMBERS;

    let shard = Arc::new(Mutex::new(Shard::new(ShardId::ONE, token.clone(), intents)));

    let http = HttpClient::new(token);

    let cache = Arc::new(InMemoryCache::builder().build());
    let client = Arc::new(Client {
        http: Arc::new(http),
        cache: Arc::clone(&cache),
        re: Arc::new(Regex::new(r"https://discord(app)?.com/channels/(\d+)/(\d+)/(\d+)").unwrap()),
        shard: Arc::clone(&shard),
    });

    while let Some(item) = {
        let mut shard = shard.lock().await;
        shard.next_event(EventTypeFlags::all()).await
    } {
        let Ok(event) = item else {
            tracing::warn!(source = ?item.unwrap_err(), "error receiving event");

            continue;
        };

        cache.update(&event);

        tokio::spawn(handle_event(event, Arc::clone(&client)));
    }

    Ok(())
}

async fn handle_event(event: Event, client: Arc<Client>) -> anyhow::Result<()> {
    match event {
        Event::MessageCreate(msg) => {
            if msg.author.bot {
                return Ok(());
            }
            if let Some(caps) = client.re.captures(msg.content.as_str()) {
                let channel_id = caps.get(3).unwrap().as_str().parse::<u64>().unwrap();
                let message_id = caps.get(4).unwrap().as_str().parse::<u64>().unwrap();

                let channel: Id<ChannelMarker> = Id::new(channel_id);
                let message: Option<MessageData> = {
                    if let Some(message) = client.cache.message(Id::new(message_id)) {
                        let channel = client.cache.channel(channel).unwrap();
                        let mut image = None;
                        if !message.attachments().is_empty() {
                            image = Some(message.attachments()[0].url.clone());
                        }
                        Some(MessageData {
                            content: message.content().to_string(),
                            author_id: message.author(),
                            channel_name: channel.name.clone().unwrap(),
                            id: message.id(),
                            image,
                        })
                    } else {
                        let message = client.http.message(channel, Id::new(message_id)).await;
                        if let Ok(message) = message {
                            let target = message.model().await?;
                            let channel = client.cache.channel(channel).unwrap();
                            let mut image = None;
                            if !target.attachments.is_empty() {
                                image = Some(target.attachments[0].url.clone());
                            }
                            Some(MessageData {
                                content: target.content.clone().to_string(),
                                author_id: target.author.id,
                                channel_name: channel.name.clone().unwrap(),
                                id: target.id,
                                image,
                            })
                        } else {
                            None
                        }
                    }
                };
                if let Some(target) = message {
                    let author = {
                        if let Some(user) = client.cache.user(target.author_id) {
                            Author {
                                name: user.name.clone(),
                                id: user.id,
                                avatar: user.avatar,
                            }
                        } else {
                            let user = client.http.user(target.author_id).await?.model().await?;
                            Author {
                                name: user.name.clone(),
                                id: user.id,
                                avatar: user.avatar,
                            }
                        }
                    };
                    let avatar_url = {
                        format!(
                            "https://cdn.discordapp.com/avatars/{}/{}.png",
                            author.id,
                            author.avatar.as_ref().unwrap()
                        )
                    };
                    let mut embed = EmbedBuilder::new()
                        .description(target.content)
                        .author(
                            EmbedAuthorBuilder::new(author.name.clone())
                                .icon_url(ImageSource::url(avatar_url).unwrap())
                                .build(),
                        )
                        .footer(EmbedFooterBuilder::new(target.channel_name).build())
                        .color(0x02caf7)
                        .timestamp(Timestamp::from_micros(target.id.timestamp() * 1000)?);
                    if let Some(image) = target.image {
                        embed = embed.image(ImageSource::url(image).unwrap());
                    };
                    let embed = embed.build();

                    client
                        .http
                        .create_message(msg.channel_id)
                        .embeds(&[embed])
                        .await?;
                };
            }
        }
        Event::Ready(_) => {
            println!("Shard is ready");
            {
                let shard = client.shard.lock().await;
                shard.command(&UpdatePresence::new(
                    vec![Activity {
                        application_id: None,
                        assets: None,
                        buttons: Vec::new(),
                        created_at: None,
                        details: None,
                        emoji: None,
                        flags: None,
                        id: None,
                        instance: None,
                        kind: ActivityType::Watching,
                        name: format!("v{}", std::env!("CARGO_PKG_VERSION")),
                        party: None,
                        secrets: None,
                        state: None,
                        timestamps: None,
                        url: None,
                    }],
                    false,
                    None,
                    Status::Online,
                )?);
            };
        }
        _ => {}
    }

    Ok(())
}
