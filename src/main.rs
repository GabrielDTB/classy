mod get_classes;

use anyhow::Result;
use get_classes::*;
use rand::Rng;
use serde_json;
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
    classes: Vec<Class>,
}
impl Handler {
    fn class_embed(class: &Class) -> CreateEmbed {
        let offered = class.offered.join("\n");
        let distribution = class.distribution.join("\n");
        CreateEmbed::default()
            .title(format!("{} {}", class.id, class.name))
            .url(format!("{}", class.link))
            .description(format!("{}", class.description))
            .fields({
                let mut fields = vec![];
                if !class.credits.is_empty() {
                    fields.push(("Credits", &*class.credits, false));
                }
                if !class.cross_listed.is_empty() {
                    fields.push(("Cross Listed Classes", &*class.cross_listed, false));
                }
                if !class.prerequisites.is_empty() {
                    fields.push(("Prerequisites", &*class.prerequisites, false));
                } else {
                    fields.push(("Prerequisites", "None", false));
                }
                if !offered.is_empty() {
                    fields.push(("Offered", &*offered, false));
                }
                if !distribution.is_empty() {
                    fields.push(("Distribution", &*distribution, false));
                }
                fields
            })
            .footer(|f| f.text("Database Years: 2022-2023"))
            .to_owned()
    }
    fn prefixes_embed(&self) -> CreateEmbed {
        let mut embed = CreateEmbed::default();
        embed.title("Class Prefixes");
        embed.description("Here are all the prefixes for classes that can be queried with classy random:");
        embed.fields({
            let mut fields = vec![];
            for class in self.classes.iter() {
                let prefix = class.id.chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>().to_ascii_uppercase();
                if !fields.iter().any(|(p, _, _)| p == &prefix) {
                    fields.push((prefix, "", true));
                }
            }
            fields.sort();
            fields
        });
        embed
    }
    fn query(&self, id: &str) -> Option<&Class> {
        for class in self.classes.iter() {
            if class
                .id
                .chars()
                .filter(|c| *c != ' ') // TODO this is a hack
                .collect::<String>()
                .eq_ignore_ascii_case(id)
            {
                return Some(class);
            }
        }
        None
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        let mut tokens = msg
            .content
            .split(" ")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase());
        match tokens.next().as_deref() {
            Some(PREFIX) => {}
            _ => return,
        }
        let statuses = match tokens.next().as_deref() {
            Some("query") => {
                let id = tokens.collect::<String>();
                let class = self.query(&id);
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
                let filters = tokens.collect::<Vec<_>>();
                let matches = self
                    .classes
                    .iter()
                    .filter(|c| {
                        filters.contains(
                            &c.id
                                .chars()
                                .filter(|c| c.is_ascii_alphabetic())
                                .collect::<String>()
                                .to_lowercase(),
                        )
                    })
                    .collect::<Vec<_>>();
                let matches = match matches.len() {
                    0 => self.classes.iter().collect::<Vec<_>>(),
                    _ => matches,
                };
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
            Some("prefixes") => { // list all the course prefixes as an embed with fields
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
    println!("Checking cache paths and creating if absent...");
    for path in ["./cache/responses", "./cache/classes"] {
        tokio::fs::create_dir_all(path).await?;
    }

    println!("Listing entries in ./cache/responses...");
    let cached_response_names = std::fs::read_dir("./cache/responses")
        .unwrap()
        .map(|d| d.unwrap().file_name().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    println!("Listing entries in ./cache/classes...");
    let cached_class_names = std::fs::read_dir("./cache/classes")
        .unwrap()
        .map(|d| d.unwrap().file_name().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    let mut classes = Vec::with_capacity(cached_class_names.len());
    if cached_class_names.len() >= cached_response_names.len() && cached_class_names.len() != 0 {
        println!(
            "Loading {} cached classes from ./cache/classes...",
            cached_class_names.len()
        );
        classes.extend(cached_class_names.iter().map(|name| {
            let file = std::fs::File::open(format!("./cache/classes/{name}")).unwrap();
            let reader = std::io::BufReader::new(file);
            serde_json::from_reader(reader).unwrap()
        }));
    } else {
        println!(
            "Loading {} cached responses from ./cache/responses...",
            cached_response_names.len()
        );
        let mut responses = cached_response_names
            .iter()
            .map(|name| {
                let file = std::fs::File::open(format!("./cache/responses/{name}")).unwrap();
                let reader = std::io::BufReader::new(file);
                serde_json::from_reader(reader).unwrap()
            })
            .collect::<Vec<_>>();

        println!("Checking for missing links in cached responses...");
        'outer: loop {
            let query = query_classes(&responses).await;
            for response in query {
                let response = match response {
                    Ok(response) => response,
                    Err(_) => continue 'outer,
                };
                if responses
                    .iter()
                    .filter(|r| (**r).link == response.link)
                    .count()
                    == 0
                {
                    responses.push(response);
                }
            }
            break;
        }

        println!(
            "Writing {} new responses to ./cache/responses...",
            responses.len() - cached_response_names.len()
        );
        for response in responses.iter() {
            let sanitized_link = response.link.replace("/", "%");
            if !cached_response_names.contains(&sanitized_link) {
                std::fs::write(
                    format!("./cache/responses/{sanitized_link}"),
                    serde_json::to_string_pretty(&response).unwrap(),
                )
                .unwrap();
            };
        }

        println!("Parsing responses into Class objects...");
        classes.extend(responses.into_iter().map(|r| parse_class(r)));

        println!(
            "Checking against {} cached classes...",
            cached_class_names.len()
        );
        for class in classes.iter() {
            let short_id = class
                .id
                .chars()
                .filter(|c| *c != ' ')
                .map(|c| c.to_lowercase().next().unwrap())
                .collect::<String>();
            if !cached_class_names.contains(&short_id) {
                std::fs::write(
                    format!("./cache/classes/{short_id}"),
                    serde_json::to_string_pretty(class).unwrap(),
                )
                .unwrap();
            }
        }
        println!("Wrote new classes to ./cache/classes...");
    }

    println!("Starting bot...");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { classes })
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?} : '{}'", why, token);
    }
    Ok(())
}
