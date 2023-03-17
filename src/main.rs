use anyhow::Result;

use rand::thread_rng;
use rand::Rng;

use serenity::model::prelude::interaction::message_component::MessageComponentInteraction;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serenity::async_trait;
use serenity::builder::{
    CreateActionRow, CreateApplicationCommand, CreateButton, CreateComponents,
};
use serenity::model::application::command::Command;
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::prelude::ReactionType;
use serenity::prelude::*;

static SCORE_TIMEOUT: u64 = 2000; // milliseconds

struct Handler {
    game_map: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
    // active_map: Arc<RwLock<HashMap<String, String, u64>>>,
}

pub fn generate_map(components: &mut CreateComponents) {
    let mut seed = thread_rng();
    let random_x = seed.gen_range(0..5);
    let random_y = seed.gen_range(0..5);

    for x in 0..5 {
        let mut action_row = CreateActionRow::default();

        for y in 0..5 {
            if x == random_x && y == random_y {
                action_row.add_button(create_button(ReactionType::Unicode("🦑".to_string())));
            } else {
                action_row.add_button(create_button(ReactionType::Unicode("⬜".to_string())));
            }
        }

        components.add_action_row(action_row);
    }
}

async fn remove_item(game_map: &mut Arc<RwLock<HashMap<String, (u32, Instant)>>>, command: &MessageComponentInteraction) {
    game_map.write().await.remove(&command.user.id.to_string());
}

// async fn exists_item(game_map: &mut tokio::sync::RwLockWriteGuard<'_, HashMap<std::string::String, (u32, std::time::Instant)>>, command: &MessageComponentInteraction) {
//     game_map.get(&command.user.id.to_string());
// }

async fn get_item(
    game_map: &Arc<tokio::sync::RwLock<HashMap<std::string::String, (u32, std::time::Instant)>>>,
    user_id: &str,
) -> Option<(u32, Instant)> {
    let game_map_guard = game_map.read().await;

    game_map_guard.get(user_id).cloned()
}

fn increase_score(game_map: &mut HashMap<String, (u32, Instant)>, user_id: &str) {
    let score = game_map
        .entry(user_id.to_string())
        .or_insert((0, Instant::now()));
    score.0 += 1;
    score.1 = Instant::now();
}

// define a function to create a single button
fn create_button(emoji: ReactionType) -> CreateButton {
    let mut button = CreateButton::default();
    button.style(ButtonStyle::Secondary);
    button.label("\u{200B}");
    button.emoji(emoji.clone());
    button.custom_id(format!(
        "{}_{}",
        thread_rng().gen::<u64>(),
        if &emoji == &ReactionType::Unicode("🦑".to_string()) {
            "_win"
        } else {
            ""
        }
    ));
    button
}

async fn check_for_game_end(
    ctx: &Context,
    interaction: &Interaction,
    game_map: &Arc<tokio::sync::RwLock<HashMap<String, (u32, std::time::Instant)>>>,
    command: &MessageComponentInteraction,
) {
    let game_map_inner = Arc::clone(game_map);
    let command_clone = command.clone();
    let ctx_clone = ctx.clone();
    let interaction_clone = interaction.clone();
    let mut game_map_cloned = game_map.clone();
    let command_cloned = command.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let score = get_item(&game_map_inner, &command_clone.user.id.to_string()).await;

        if let Some((final_score, timestamp)) = score {
            let difference = Instant::now().duration_since(timestamp);

            if difference.as_millis() > SCORE_TIMEOUT.into() {
                if let Err(reason) = interaction_clone
                    .message_component()
                    .unwrap()
                    .message
                    .edit(ctx_clone.http.clone(), |message| {
                        message.content(format!(
                            "💀 Too slow! No score within `{:?}` ms! Score: **{:?}** :star:",
                            SCORE_TIMEOUT, final_score
                        ));
                        message.components(|components| components);
                        message
                    })
                    .await
                {
                    println!("{:?}", reason)
                } else {
                    remove_item(&mut game_map_cloned, &command_cloned).await;
                };
            }
        }
    });
}

async fn interaction_handler(
    ctx: Context,
    interaction: Interaction,
    handler: Arc<&Handler>,
) -> Result<()> {
    match interaction.clone() {
        Interaction::MessageComponent(command) => {
            // let message_id = interaction.id();

            // let mut game_map = handler.game_map.write().await;
            // if let data_exists = exists_item(&mut game_map, &command) {
            //     //
            // } else {
            //     return Ok(());
            // }
            command
                .create_interaction_response(&ctx.http, |response| {
                    response.kind(InteractionResponseType::DeferredUpdateMessage)
                })
                .await?;

            let custom_id = &command.data.custom_id;
            if custom_id.ends_with("_win") {
                let mut game_map = handler.game_map.write().await;
                increase_score(&mut game_map, &command.user.id.to_string());
            } else {
                // no need to regenerate
                return Ok(());
            }

            let game_map_tokio = handler.game_map.clone();

            let game_map = Arc::clone(&handler.game_map);

            check_for_game_end(&ctx, &interaction, &game_map, &command).await;

            let data = get_item(&game_map_tokio, &command.user.id.to_string()).await;

            interaction
                .message_component()
                .unwrap()
                .message
                .edit(ctx.http.clone(), |message| {
                    if let Some(score) = data {
                        message.content(format!("Your score: **{:?}** :star:", score.0));
                    }

                    message.components(|components| {
                        generate_map(components);
                        components
                    });
                    message
                })
                .await
                .unwrap();
        }
        Interaction::ApplicationCommand(command) => {
            // modify your existing code to create a 5x5 button grid using the create_button_row function

            if command.data.name == "whac-a-hole" {
                command
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.components(|components| {
                                    generate_map(components);
                                    components
                                });
                                message
                            })
                    })
                    .await?;
                // let message =
                //     ApplicationCommandInteraction::get_interaction_response(&command, &ctx.http)
                //         .await
                //         .unwrap();

                // let active_map = &mut handler.active_map.read().await;

                // active_map.insert(command.user.id.to_string(), message.id.as_u64());
            }
        }
        _ => {}
    }
    Ok(())
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let handler = Arc::new(self.clone());
        let result = interaction_handler(ctx, interaction, handler).await;
        if let Err(e) = result {
            println!("Error running interaction_handler: {e:?}");
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        if let Err(reason) = Command::create_global_application_command(
            &ctx.http,
            |command: &mut CreateApplicationCommand| {
                command
                    .name("whac-a-hole")
                    .description("Play Whac-A-Hole in Discord!")
            },
        )
        .await
        {
            println!("Caught error: {:?}", reason)
        };
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let token = "";

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            game_map: Arc::new(RwLock::new(HashMap::new())),
            // active_map: Arc::new(RwLock::new(HashMap<std::string::String)),
        })
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
