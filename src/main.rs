mod get_courses;

use std::env;

use anyhow::{bail, Result};
use get_courses::Course;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::collections::HashSet;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if msg.content.starts_with("!classy") {
            let channel = match msg.channel_id.to_channel(&context).await {
                Ok(channel) => channel,
                Err(why) => {
                    println!("Error getting channel: {:?}", why);

                    return;
                }
            };
            let mut response: String;
            let command = msg.content.split_once("!class").unwrap().1.trim();
            if command.starts_with("query") {
                let body = command.split_once("query").unwrap().1.trim();
                let body = body
                    .chars()
                    .filter(|c| *c != ' ' && *c != ',' && *c != '-' && *c != '_')
                    .collect::<String>()
                    .to_uppercase();
            }
            // The message builder allows for creating a message by
            // mentioning users dynamically, pushing "safe" versions of
            // content (such as bolding normalized content), displaying
            // emojis, and more.
            let response = MessageBuilder::new()
                .push("User ")
                .push_bold_safe(&msg.author.name)
                .push(" used the 'ping' command in the ")
                .mention(&channel)
                .push(" channel")
                .build();

            if let Err(why) = msg.channel_id.say(&context.http, &response).await {
                println!("Error sending message: {:?}", why);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    get_courses::do_stuff().await?;
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
    Ok(())
}

fn query_course(courses: HashSet<Course>, quarry: &str) -> Option<Course> {
    // Separate course prefix from numbers and reorder to allow for more acceptable queries
    let mut prefix = Vec::with_capacity(8);
    let mut numbers = Vec::with_capacity(3);
    for char in quarry
        .to_ascii_uppercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
    {
        match char.is_digit(10) {
            true => numbers.push(char),
            false => prefix.push(char),
        }
    }
    prefix.push(' ');
    prefix.append(&mut numbers);
    let id = prefix.into_iter().collect::<String>();
    courses.get(&id)
}

fn format_for_discord(course: Course) -> String {}
