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
const CURRENT_YEARS: &str = "2023-2024";

struct Handler {
    catalog: Catalog,
}

impl Handler {
    fn class_embed(&self, class: &Class) -> CreateEmbed {
        CreateEmbed::default()
            .title(format!("{} {}", class.id(), class.title()))
            .url(class.url())
            .description(class.description())
            .fields({
                let mut fields = vec![];
                fields.push(("Credits", class.credits().to_string(), false));
                let cross_listings = class.cross_listings().join(", ").trim().to_owned();
                if !cross_listings.is_empty() {
                    fields.push(("Cross Listed Classes", cross_listings, false));
                }
                if !class.prerequisites().is_empty() {
                    fields.push(("Prerequisites", class.prerequisites().to_owned(), false));
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
            .footer(|f| f.text(format!("Years: {CURRENT_YEARS} -- Classes: {}", self.catalog.query_by_department("").len())))
            .color(STEVENS_RED)
            .to_owned()
    }
    fn class_list_embed(&self, classes: Vec<&Class>) -> Option<CreateEmbed> {
        if classes.len() > 25 || classes.is_empty() {
            return None;
        }
        fn format_description(description: &str) -> String {
            let description = description.chars().collect::<Vec<char>>();
            let max_length = 135;
            if description.len() <= max_length {
                return description.iter().collect::<String>();
            }
            let shortened = description[..max_length-4].iter().rev().collect::<String>();
            let split = match shortened.split_once(" ") {
                Some(split) => split.1,
                None => &*shortened,
            };
            let reassembled = split.chars().rev().collect::<String>();
            format!("{reassembled} ...")
        }
        let fields = classes.iter().map(|c| (format!("{} {}", c.id(), c.title()), format!("{} [[^]]({})", format_description(&c.description()), c.url()), false)).collect::<Vec<_>>();
        Some(CreateEmbed::default()
            .fields(fields)
            .footer(|f| f.text(format!("Years: {CURRENT_YEARS} -- Classes: {}", self.catalog.query_by_department("").len())))
            .color(STEVENS_RED)
            .to_owned())
    }
    fn departments_embed(&self) -> CreateEmbed {
        CreateEmbed::default()
            .title("Class Departments")
            .description(
                self.catalog.departments().into_iter().map(|t| format!("**{}:** {}\n", t.0, t.1)).collect::<String>().trim()
            )
            .color(STEVENS_RED)
            .to_owned()
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        let mut tokens = msg
            .content
            .split(" ")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase());
        match tokens.next().as_deref() {
            Some(PREFIX) => {}
            _ => return,
        }
        let statuses = match tokens.next().as_deref() {
            Some("query" | "q") => {
                let id = tokens.collect::<String>();
                let class = self.catalog.query_by_id(&id);
                let embed = match class {
                    Some(class) => {
                        let mut embed = self.class_embed(class);
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
                    msg.reply(&context.http, format!(r#"Class "{id}" not found. Does it exist?"#))
                        .await
                }]
            }
            Some("random" | "rand" | "r") => {
                let mut departments = tokens.collect::<Vec<String>>();
                if departments.is_empty() {
                    departments.push(String::from(""));
                }
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
                                    "No classes found for departments [{}]. Do those departments exist?",
                                    departments.join(", ")
                                ),
                            )
                            .await,
                    ]
                } else {
                    let class = matches
                        .get(rand::thread_rng().gen_range(0..matches.len()))
                        .unwrap();
                    let embed = self.class_embed(class).color(STEVENS_RED).to_owned();
                    vec![
                        msg.channel_id
                            .send_message(&context.http, |m| m.set_embed(embed))
                            .await,
                    ]
                }
            }
            Some("help" | "h") => {
                vec![
                    msg.reply(
                        &context.http,
                        // There's gotta be a better way to format this
                        "\
                        __**Commands**__\n\
                        **help**\n\
                          \tGives this message.\n\
                        **query** __class_id__\n\
                          \tGives details about a class.\n\
                          \t*Examples*\n\
                            \t\tclassy query cs 115\n\
                            \t\tclassy query ma125\n\
                        **random** __class_prefix__ __...__\n\
                          \tQueries a random class from the given prefixes.\n\
                          \t*Defaults*\n\
                            \t\tIf no class prefix is supplied, a random\n\
                            \t\tclass from all available classes is picked.\n\
                          \t*Examples*\n\
                            \t\tclassy random\n\
                            \t\tclassy random hli\n\
                            \t\tclassy random cs cpe ee\n\
                        **departments**\n\
                          \tLists all the class departments\n\
                          \tused for class queries.\n\
                        **aliases**\n\
                          \tLists all the aliases for each command.\n\
                        **calendar**\n\
                          \tReturns the link to the current/upcoming\n\
                          \tyear's academic calendar.\n\
                        **search** __query__\n\
                          \tReturns the top 10 classes for a query.\n\
                          \t*Examples*\n\
                            \t\tclassy search linear algebra\n\
                            \t\tclassy search compilers\n\
                        "
                        .trim(),
                    )
                    .await,
                ]
            }
            Some("aliases" | "a") => {
                vec![
                    msg.reply(
                        &context.http,
                        "\
                        __**Command Aliases**__\n\
                        **help:** h\n\
                        **query:** q\n\
                        **random:** rand, r\n\
                        **departments:** dep, d\n\
                        **aliases:** a\n\
                        **calendar:** c\n\
                        **search:** a\n\
                        "
                        .trim(),
                    )
                    .await,
                ]
            }
            Some("departments" | "dep" | "d") => {
                // list all the course prefixes as an embed with fields
                vec![
                    msg.channel_id
                        .send_message(&context.http, |m| m.set_embed(self.departments_embed()))
                        .await,
                ]
            }
            Some("calendar" | "c") => {
                vec![
                    msg.reply(
                        &context.http, 
                        format!("Here is the link for the {CURRENT_YEARS} academic calendar: https://assets.stevens.edu/mviowpldu823/5UlooMY3Cp7TtZctposW1C/d33d938e36645b08425ae48f1844244e/2023-2024_Academic_Calendar03192023__1_.pdf"),
                    ).await,
                ]
            }
            Some("search" | "s") => {
                let query = tokens.collect::<Vec<String>>().join(" ");
                let matches = self.catalog.search(&query, 10);
                match self.class_list_embed(matches) {
                    Some(mut embed) => vec![
                        msg.channel_id
                            .send_message(&context.http, |m| m.set_embed((&mut embed).title(format!(r#"Results for "{query}""#)).to_owned()))
                            .await
                    ],
                    None => vec![msg.channel_id.say(&context.http, format!(r#"No results for "{query}""#)).await],
                }
            },
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
