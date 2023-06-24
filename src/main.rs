mod get_courses;

use anyhow::Result;
// use async_once::AsyncOnce;
use futures::future::join_all;
use get_courses::*;
// use lazy_static::lazy_static;
// use rand::Rng;
use serenity::async_trait;
use serenity::builder::CreateEmbed;
use serenity::model::channel::*;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
// use std::collections::HashMap;
use std::env;
// use thiserror::Error;

const PREFIX: &str = "classy";

struct Handler {
    classes: Vec<Class>,
}
impl Handler {
    fn class_embed(class: &Class) -> CreateEmbed {
        let mut embed = CreateEmbed::default();
        embed
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
                    fields.push(("Prerequisites", "None".into(), false));
                }
                if !class.offered.is_empty() {
                    fields.push(("Offered", &*class.offered, false));
                }
                if !class.distribution.is_empty() {
                    fields.push(("Distribution", &*class.distribution, false));
                }
                fields
            })
            .footer(|f| f.text("Database updated"))
            .timestamp("2023-05-19T19:00:02Z");
        embed
    }
    fn query(&self, id: &str) -> Option<&Class> {
        for class in self.classes.iter() {
            if class.id.eq_ignore_ascii_case(id) {
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
        let errors = match tokens.next().as_deref() {
            Some("query") => {
                let ids = tokens.collect::<Vec<_>>();
                let classes = ids.iter().map(|id| self.query(id)).collect::<Vec<_>>();
                let embeds = classes
                    .iter()
                    .map(|class| match class {
                        Some(class) => {
                            let mut embed = Handler::class_embed(class);
                            embed.author(|a| a.name(msg.author.clone()));
                            Some(embed)
                        }
                        None => None,
                    })
                    .collect::<Vec<_>>();

                join_all(ids.iter().zip(embeds).map(|(id, embed)| async {
                    match embed {
                        Some(embed) => {
                            msg.channel_id
                                .send_message(&context.http, |m| m.set_embed(embed))
                                .await
                        }
                        None => {
                            msg.channel_id
                                .send_message(&context.http, |m| {
                                    m.content(format!(r#"Class "{}" not found"#, *id))
                                })
                                .await
                        }
                    }
                }))
                .await
                .into_iter()
                .filter_map(|r| match r {
                    Err(why) => Some(why),
                    Ok(_) => None,
                })
                .collect::<Vec<_>>()
            }
            Some("help") => todo!(),
            _ => return,
        };

        for error in errors {
            println!("{:?}", error);
        }
        // else if command.starts_with("help") {
        //     if let Err(why) = msg.channel_id.say(&context.http, format!("Use the \"query\" command followed by the course id (eg. CS 115) to get details about a course.")).await {
        //             println!("Error sending message: {:?}", why);
        //         };
        // } else if command.starts_with("random") {
        //     let args = command
        //         .split_once("random")
        //         .unwrap()
        //         .1
        //         .split(" ")
        //         .filter(|s| !s.is_empty())
        //         .map(|s| s.to_uppercase())
        //         .collect::<Vec<_>>();
        //     let keys = match args.len() {
        //         0 => COURSES.get().await.keys().collect::<Vec<_>>(),
        //         _ => COURSES
        //             .get()
        //             .await
        //             .keys()
        //             .filter(|k| {
        //                 for arg in args.iter() {
        //                     if k.split(" ").next().unwrap() == arg {
        //                         return true;
        //                     }
        //                 }
        //                 false
        //             })
        //             .collect::<Vec<_>>(),
        //     };
        //     if keys.len() == 0 {
        //         if let Err(why) = msg
        //             .channel_id
        //             .say(
        //                 &context.http,
        //                 format!(
        //                     "No courses matched the arguments \"{}\"",
        //                     command.split_once("random").unwrap().1.trim()
        //                 ),
        //             )
        //             .await
        //         {
        //             println!("Error sending message: {:?}", why);
        //         };
        //     } else {
        //         let r = rand::thread_rng().gen_range(0..keys.len() - 1);
        //         let key = keys.iter().skip(r).next().unwrap();
        //         msg.content = format!("classy query {}", key);
        //         self.message(context, msg).await;
        //     }
        // }
    }
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let responses = query_classes(Vec::new()).await;
    let classes = responses
        .into_iter()
        .filter_map(|r| match r {
            Ok(class) => Some(parse_class(class)),
            Err(_) => None,
        })
        .collect::<Vec<_>>();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { classes })
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
    Ok(())
}
