mod catalog;
mod class;
mod get_classes;
mod traits;

use anyhow::Result;
use catalog::*;
use class::*;
use rand::Rng;
use serenity::async_trait;
use serenity::builder::CreateEmbed;
use serenity::model::channel::*;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::env;
// use thiserror::Error;

const PREFIX: &str = "classy";
const STEVENS_RED: serenity::utils::Color = serenity::utils::Color::from_rgb(163, 35, 56);

struct Handler {
    catalog: Catalog,
}

impl Handler {
    fn class_embed(class: &Class) -> CreateEmbed {
        let offered = class.offered().join("\n");
        let distributions = class.distributions().join("\n");
        CreateEmbed::default()
            .title(class.id())
            .url(class.url())
            .description(class.description())
            .fields({
                let mut fields = vec![];
                fields.push(("Credits", class.credits().to_string(), false));
                let cross_listings = class.cross_listings().join("\n").trim().to_owned();
                if !cross_listings.is_empty() {
                    fields.push(("Cross Listed Classes", cross_listings, false));
                }
                if !class.prerequisites().is_empty() {
                    fields.push(("Prerequisites", class.prerequisites(), false));
                } else {
                    fields.push(("Prerequisites", String::from("None"), false));
                }
                let offered = class.offered().join("\n").trim().to_owned();
                if !offered.is_empty() {
                    fields.push(("Offered", offered, false));
                }
                let distributions = class.distributions().join("\n").trim().to_owned();
                if !distributions.is_empty() {
                    fields.push(("Distribution", distributions, false));
                }
                fields
            })
            .footer(|f| f.text("Database Years: 2022-2023"))
            .to_owned()
    }
    fn prefixes_embed(&self) -> CreateEmbed {
        let mut embed = CreateEmbed::default();
        embed.title("Class Prefixes");
        embed.description(
            "Here are all the prefixes for classes that can be queried with classy random:",
        );
        embed.fields({
            let mut departments = self
                .catalog
                .departments()
                .iter()
                .map(|d| (d, "", true))
                .collect::<Vec<(&String, &str, bool)>>();
            departments.sort();
            departments
        });
        embed
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        let mut tokens = msg
            .content
            .split(" ")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_uppercase());
        match tokens.next().as_deref() {
            Some(PREFIX) => {}
            _ => return,
        }
        let statuses = match tokens.next().as_deref() {
            Some("query") => {
                let id = tokens.collect::<String>();
                let class = self.catalog.query_by_id(&id);
                let embed = match class {
                    Some(class) => {
                        let mut embed = Handler::class_embed(class);
                        embed.color(STEVENS_RED);
                        Some(embed)
                    }
                    None => None,
                };
                vec![if let Some(embed) = embed {
                    msg.channel_id
                        .send_message(&context.http, |m| m.set_embed(embed))
                        .await
                } else {
                    msg.reply(&context.http, format!(r#"Class "{id}" not found"#))
                        .await
                }]
            }
            Some("random") => {
                let mut departments = tokens.collect::<Vec<String>>();
                if departments.is_empty() {
                    departments.push(String::from("*"));
                }
                let matches = departments
                    .iter()
                    .flat_map(|d| self.catalog.query_by_department(d))
                    .collect::<Vec<&Class>>();
                let matches = departments
                    .iter()
                    .fold(Vec::new(), |mut matches, department| {
                        matches.extend(self.catalog.query_by_department(department));
                        matches.sort_unstable_by_key(|c| c.id());
                        matches.dedup_by_key(|c| c.id());
                        matches
                    });
                if matches.is_empty() {
                    vec![
                        msg.channel_id
                            .say(
                                &context.http,
                                format!(
                                    "No classes exist within departments: {}",
                                    departments.join(", ")
                                ),
                            )
                            .await,
                    ]
                } else {
                    let class = matches
                        .get(rand::thread_rng().gen_range(0..matches.len()))
                        .unwrap();
                    let embed = Handler::class_embed(class).color(STEVENS_RED).to_owned();
                    vec![
                        msg.channel_id
                            .send_message(&context.http, |m| m.set_embed(embed))
                            .await,
                    ]
                }
            }
            Some("help") => {
                vec![
                    msg.reply(
                        &context.http,
                        // There's gotta be a better way to format this
                        "\
                        __**Commands**__\n\
                        **help**\n\t\
                            Gives this message.\n\
                        **query** __class_id__\n\t\
                            Gives details about a class.\n\t\
                            *Examples*\n\t\t\
                                classy query cs 115\n\t\t\
                                classy query ma125\n\
                        **random** __class_prefix__ __...__\n\t\
                            Queries a random class from the given prefixes.\n\t\
                            *Defaults*\n\t\t\
                                If no class prefix is supplied, a random\n\t\t\
                                class from all available classes is picked.\n\t\
                            *Examples*\n\t\t\
                                classy random\n\t\t\
                                classy random hli\n\t\t\
                                classy random cs cpe ee\n\
                            **prefixes**\n\t\
                            Lists all the class prefixes.
                        ",
                    )
                    .await,
                ]
            }
            Some("prefixes") => {
                // list all the course prefixes as an embed with fields
                vec![
                    msg.channel_id
                        .send_message(&context.http, |m| m.set_embed(self.prefixes_embed()))
                        .await,
                ]
            }
            _ => return,
        };

        for status in statuses {
            match status {
                Ok(_) => {}
                Err(why) => println!("{:?}", why),
            }
        }
    }
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let catalog = Catalog::new_filled().await?;
    println!("Starting bot...");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { catalog })
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?} : '{}'", why, token);
    }
    Ok(())
}
