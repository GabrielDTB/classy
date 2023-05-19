mod get_courses;

use std::env;

use anyhow::{bail, Result};
use get_courses::Course;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::cell::RefCell;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CourseQueryError {
    #[error("course distribution `{distribution}` not found in course list")]
    DistributionNotFound { distribution: String },
    #[error("course `{course_id}` is not available")]
    CourseNotFound { course_id: String },
}

struct Handler;

const PREFIX: &str = "!classy";
thread_local! {
    static COURSES: RefCell<HashMap<String, Course>> = RefCell::new(HashMap::new());
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if msg.content.starts_with(PREFIX) {
            let channel = match msg.channel_id.to_channel(&context).await {
                Ok(channel) => channel,
                Err(why) => {
                    println!("Error getting channel: {:?}", why);

                    return;
                }
            };
            let command = msg.content.split_once(PREFIX).unwrap().1.trim();
            if command.starts_with("query") {
                let body = command.split_once("query").unwrap().1.trim();
                let response = match query_course(&*body) {
                    Ok(course) => format_for_discord(&course),
                    _ => "Couldn't find course".to_owned(),
                };
                if let Err(why) = msg.channel_id.say(&context.http, &response).await {
                    println!("Error sending message: {:?}", why);
                }
            } else {
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
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let courses = get_courses::do_stuff().await?;
    COURSES.with(|refer| {
        let mut c = refer.borrow_mut();
        for key in courses.keys() {
            c.insert(key.to_owned(), courses.get(key).unwrap().clone());
        }
        ()
    });
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

fn query_course(quarry: &str) -> Result<Course, CourseQueryError> {
    let mut r = Err(CourseQueryError::CourseNotFound {
        course_id: "".into(),
    });
    COURSES.with(|refer| {
        let courses = refer.borrow();
        // Separate course prefix from numbers and reorder to allow for more acceptable queries
        let letters = quarry
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_lowercase())
            .collect::<String>();
        let numbers = quarry
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>();
        if let Some(course) = courses.get(&(format!("{letters} {numbers}"))) {
            r = Ok(course.clone());
        } else if !courses.keys().any(|s| s.contains(&letters)) {
            r = Err(CourseQueryError::DistributionNotFound {
                distribution: letters,
            });
        } else {
            r = Err(CourseQueryError::CourseNotFound {
                course_id: letters + " " + &*numbers,
            });
        }
    });
    r
}

fn format_for_discord(course: &Course) -> String {
    let name = format!("**{} -- {}**", course.id, course.name);
    let description = format!("*{}*", course.description);
    let credits = format!("Credits: {}", course.id);
    let prerequisites = format!("Prerequisites: {:?}", course.prerequisites);
    let offered = format!("Offered: {:?}", course.offered);
    let distribution = format!("Distribution: {:?}", course.distribution);
    let link = &course.link;
    format!(
        "{name}\n\n{description}\n\n{credits}\n{prerequisites}\n{offered}\n{distribution}\n{link}"
    )
}
