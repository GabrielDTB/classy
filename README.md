# Classy for Stevens Institute of Technology. 

Classy is a helpful discord bot for getting information about classes. If you would like to add it to your server, message me [(rustaceous)](discordapp.com/users/498248505538904076) on Discord.

## Self hosting the bot

Supply your Discord bot token as the DISCORD_TOKEN environment variable. 

Responses and parsed classes are cached in ./cache. If you must delete the cache, startup will take some time to query all the courses since the requests are asynchronous but sequential. After a successful startup, responses and classes will become cached for the next restart.

## Contributing

Set up your rust environment and ensure that you can successfully `cargo run`. If you get an error that your discord token is missing, see [self hosting the bot](#self-hosting-the-bot). 

### The application logic is currently broken up as follows:

main.rs -- Handles the bot logic  
catalog.rs -- Provides all course database interaction and initialization logic  
class.rs -- Provides an interface for single classes  
get_classes.rs -- Needs refactoring but this handles the internet-catalog facing logic of querying classes for now  
traits.rs -- Provides a more general interface that derivatives should provide  

Pull requests welcome. Please interact with an open issue before taking it on, or open a new issue if one does not exist yet!

****

#### License

<sup>
Licensed under <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your discretion.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>

<div align="right"><sub><sup>(Not affiliated in any way with Stevens Institute of Technology).</sup></sub></div>
