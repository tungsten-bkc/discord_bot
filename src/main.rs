use dotenv::dotenv;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::application::command::{Command, CommandOptionType};
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "pong!").await?;
    Ok(())
}

#[group]
#[commands(ping)]
struct General;

struct RecruitData {
    title: String,
    remaining_count: i64,
}

struct Handler {
    recruit_map: Arc<Mutex<HashMap<String, RecruitData>>>,
}

impl Handler {
    fn new() -> Handler {
        Handler {
            recruit_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let _commands = Command::set_global_application_commands(&ctx.http, |commands| {
            commands.create_application_command(|command| {
                command
                    .name("recruit")
                    .description("メンバーを募集します")
                    .create_option(|option| {
                        option
                            .name("recruit_title")
                            .description("募集したいものを選択してください")
                            .kind(CommandOptionType::String)
                            .required(true)
                            .add_string_choice("StreetFighter 6", "StreetFighter 6")
                            .add_string_choice("Valorant", "Valorant")
                            .add_string_choice("OverWatch 2", "OverWatch 2")
                            .add_string_choice("映画鑑賞", "映画鑑賞")
                            .add_string_choice("なんでも", "なんでも")
                    })
                    .create_option(|option| {
                        option
                            .name("recruiting_count")
                            .description("人数を入力してください")
                            .kind(CommandOptionType::Integer)
                            .required(true)
                    })
            })
        })
        .await
        .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                if command.data.name.as_str() == "recruit" {
                    handle_recruit_command(&ctx, &command, &self.recruit_map).await;
                }
            }
            Interaction::MessageComponent(component) => {
                let custom_id = component.data.custom_id.clone();
                let mut recruit_map = self.recruit_map.lock().await;

                if let Some(recruit_data) = recruit_map.get_mut(&custom_id) {
                    recruit_data.remaining_count -= 1;

                    if recruit_data.remaining_count > 0 {
                        if let Err(err) = component
                            .create_interaction_response(&ctx.http, |response| {
                                let text = format!(
                                    "{}さんが参加します。のこり{}人募集しています。",
                                    component.user.name, recruit_data.remaining_count
                                );
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(text))
                            })
                            .await
                        {
                            println!("Error sending response: {:?}", err);
                        }
                    } else {
                        if let Err(err) = component
                            .create_interaction_response(&ctx.http, |response| {
                                let text = format!(
                                    "{}さんが参加します。のこり0人です。募集を締め切ります。",
                                    component.user.name
                                );
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(text))
                            })
                            .await
                        {
                            println!("Error sending response: {:?}", err);
                        }

                        if let Err(err) = component.message.delete(&ctx.http).await {
                            println!("Error deleting message: {:?}", err);
                        }
                        recruit_map.remove(&custom_id);
                    }
                } else if custom_id.ends_with("_cancel") {
                    let recruit_id = custom_id.trim_end_matches("_cancel");

                    if let Some(recruit_data) = recruit_map.remove(recruit_id) {
                        if let Err(err) = component
                            .create_interaction_response(&ctx.http, |response| {
                                let text = format!("{}の募集を停止しました。", recruit_data.title);
                                response
                                    .kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| message.content(text))
                            })
                            .await
                        {
                            println!("Error sending cancel response: {:?}", err);
                        }

                        if let Err(err) = component.message.delete(&ctx.http).await {
                            println!("Error deleting message: {:?}", err);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

async fn handle_recruit_command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    recruit_map: &Arc<Mutex<HashMap<String, RecruitData>>>,
) {
    let recruit_title = command
        .data
        .options
        .get(0)
        .and_then(|opt| opt.value.as_ref().map(|v| v.as_str().unwrap()))
        .unwrap_or("未設定");
    let recruiting_count = command
        .data
        .options
        .get(1)
        .and_then(|opt| opt.value.as_ref().map(|v| v.as_i64().unwrap()))
        .unwrap_or(0);

    let recruit_id = Uuid::new_v4().to_string();

    recruit_map.lock().await.insert(
        recruit_id.clone(),
        RecruitData {
            title: recruit_title.to_string(),
            remaining_count: recruiting_count,
        },
    );

    let role_mention = if recruit_title == "なんでも" {
        "@everyone".to_string()
    } else {
        if let Some(guild_id) = command.guild_id {
            match ctx.http.get_guild(guild_id.into()).await {
                Ok(guild) => {
                    // ロールを名前で検索
                    let role = guild.roles.values().find(|role| role.name == recruit_title);
                    match role {
                        Some(role) => format!("<@&{}>", role.id),
                        None => "該当するロールが見つかりません".to_string(),
                    }
                }
                Err(_) => "ギルド情報の取得に失敗しました".to_string(),
            }
        } else {
            "ギルド情報が見つかりません".to_string()
        }
    };

    let content = format!(
        "{}\nメンバー募集\n募集タイトル： {}, 人数： {}人",
        role_mention, recruit_title, recruiting_count
    );

    if let Err(err) = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.content(content).components(|components| {
                        components.create_action_row(|row| {
                            row.create_button(|button| {
                                button
                                    .custom_id(recruit_id.clone())
                                    .label("参加する")
                                    .style(ButtonStyle::Primary)
                            })
                            .create_button(|button| {
                                button
                                    .custom_id(format!("{}_cancel", recruit_id))
                                    .label("募集を中断")
                                    .style(ButtonStyle::Danger)
                            })
                        })
                    })
                })
        })
        .await
    {
        println!("Error sending interaction response: {:?}", err);
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler::new())
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
