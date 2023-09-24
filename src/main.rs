use regex::Regex;
use std::{env, error::Error, sync::Arc};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_http::Client as HttpClient;
use twilight_model::{
    id::{
        marker::{ChannelMarker, UserMarker},
        Id,
    },
    util::ImageHash,
};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

struct ClientData {
    re: Regex,
}

struct Client {
    http: Arc<HttpClient>,
    cache: Arc<InMemoryCache>,
    data: Arc<ClientData>,
}

struct MessageData {
    content: String,
    author_id: Id<UserMarker>,
    channel_name: String,
}

struct Author {
    name: String,
    id: Id<UserMarker>,
    avatar: Option<ImageHash>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let token = env::var("DISCORD_TOKEN")?;
    let intents = Intents::GUILD_MESSAGES
        | Intents::GUILDS
        | Intents::MESSAGE_CONTENT
        | Intents::GUILD_MEMBERS;

    let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);

    let http = Arc::new(HttpClient::new(token));

    let cache = Arc::new(
        InMemoryCache::builder()
            // .resource_types(ResourceType::MESSAGE | ResourceType::MEMBER | ResourceType::CHANNEL)
            .build(),
    );
    let client = Arc::new(Client {
        http: Arc::clone(&http),
        cache: Arc::clone(&cache),
        data: Arc::new(ClientData {
            re: Regex::new(r"https://discord(app)?.com/channels/(\d+)/(\d+)/(\d+)").unwrap(),
        }),
    });

    loop {
        let event = match shard.next_event().await {
            Ok(event) => event,
            Err(source) => {
                if source.is_fatal() {
                    break;
                }

                continue;
            }
        };
        cache.update(&event);

        tokio::spawn(handle_event(event, Arc::clone(&client)));
    }

    Ok(())
}

async fn handle_event(
    event: Event,
    client: Arc<Client>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) => {
            if msg.author.bot {
                return Ok(());
            }
            if let Some(caps) = client.data.re.captures(msg.content.as_str()) {
                let channel_id = caps.get(3).unwrap().as_str().parse::<u64>().unwrap();
                let message_id = caps.get(4).unwrap().as_str().parse::<u64>().unwrap();

                let channel: Id<ChannelMarker> = Id::new(channel_id);
                let message: Option<MessageData> = {
                    if let Some(message) = client.cache.message(Id::new(message_id)) {
                        let channel = client.cache.channel(channel).unwrap();
                        Some(MessageData {
                            content: message.content().clone().to_string(),
                            author_id: message.author(),
                            channel_name: channel.name.clone().unwrap(),
                        })
                    } else {
                        let message = client.http.message(channel, Id::new(message_id)).await;
                        if let Ok(message) = message {
                            let target = message.model().await?;
                            let channel = client.cache.channel(channel).unwrap();
                            Some(MessageData {
                                content: target.content.clone().to_string(),
                                author_id: target.author.id,
                                channel_name: channel.name.clone().unwrap(),
                            })
                        } else {
                            None
                        }
                    }
                };
                let target = message.unwrap();
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
                let embed = EmbedBuilder::new()
                    .description(target.content)
                    .author(
                        EmbedAuthorBuilder::new(author.name.clone())
                            .icon_url(ImageSource::url(avatar_url).unwrap())
                            .build(),
                    )
                    .footer(EmbedFooterBuilder::new(target.channel_name).build())
                    .color(0x02caf7)
                    .build();

                client
                    .http
                    .create_message(msg.channel_id)
                    .embeds(&[embed])?
                    .await?;
            }
        }
        Event::Ready(_) => {
            println!("Shard is ready");
        }
        _ => {}
    }

    Ok(())
}
