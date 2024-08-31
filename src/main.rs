use application_command::ApplicationCommandOptionType;
use message_component::ButtonStyle;
use serenity::{
    async_trait,
    client::{bridge::gateway::GatewayIntents, Context, EventHandler},
    model::{
        interactions::{
            application_command::ApplicationCommandInteraction, InteractionResponseType,
        },
        prelude::*,
    },
    prelude::*,
};

use std::collections::HashMap;

// Botのイベントハンドラを定義
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        // スラッシュコマンドをサーバーに登録
        GuildId(ready.guilds[0].id().0)
            .create_application_command(&ctx.http, |command| {
                command
                    .name("recruit")
                    .description("メンバーを募集する")
                    .create_option(|option| {
                        option
                            .name("人数")
                            .description("募集する人数")
                            .kind(ApplicationCommandOptionType::Integer)
                            .required(true)
                    })
                    .create_option(|option| {
                        option
                            .name("ゲーム")
                            .description("遊びたいゲームタイトル")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    })
                    .create_option(|option| {
                        option
                            .name("コメント")
                            .description("メッセージに追加するコメント（任意）")
                            .kind(ApplicationCommandOptionType::String)
                            .required(false)
                    })
            })
            .await
            .expect("Could not create slash command");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            if command.data.name == "recruit" {
                handle_recruit_command(&ctx, &command).await;
            }
        }
    }
}

// 募集コマンドの処理関数
async fn handle_recruit_command(ctx: &Context, command: &ApplicationCommandInteraction) {
    // 引数の取得
    let num_needed = command
        .data
        .options
        .get(0)
        .and_then(|opt| opt.value.as_ref())
        .unwrap()
        .as_i64()
        .unwrap();
    let game_title = command
        .data
        .options
        .get(1)
        .and_then(|opt| opt.value.as_ref())
        .unwrap()
        .as_str()
        .unwrap();
    let comment = command
        .data
        .options
        .get(2)
        .and_then(|opt| opt.value.as_ref())
        .map(|val| val.as_str().unwrap())
        .unwrap_or("");

    // 募集メッセージの送信
    let response_content = format!(
        "@{}さんが{}人募集しています。\n{}",
        game_title, num_needed, comment
    );

    // メッセージを返信
    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message
                        .content(response_content)
                        .allowed_mentions(|mentions| {
                            mentions.empty_parse() // メンションのパースを空にして安全に処理する
                        })
                        .components(|components| {
                            components.create_action_row(|row| {
                                row.create_button(|button| {
                                    button
                                        .label("参加")
                                        .style(ButtonStyle::Success)
                                        .custom_id("participate")
                                })
                                .create_button(|button| {
                                    button
                                        .label("キャンセル")
                                        .style(ButtonStyle::Danger)
                                        .custom_id("cancel")
                                })
                            })
                        })
                })
        })
        .await
        .unwrap();
}

// Recruitmentデータ構造体の定義
struct Recruitment {
    author: UserId,
    game_title: String,
    num_needed: i64,
    participants: Vec<UserId>,
}

// BotData構造体の定義
struct BotData {
    recruitments: serenity::prelude::Mutex<HashMap<MessageId, Recruitment>>, // serenity::prelude::Mutexを使用
}

// TypeMapKeyトレイトをBotDataに実装する
impl TypeMapKey for BotData {
    type Value = serenity::prelude::Mutex<HashMap<MessageId, Recruitment>>;
}

#[tokio::main]
async fn main() {
    // Discordのゲートウェイインテントを設定
    let intents = GatewayIntents::GUILD_MEMBERS | GatewayIntents::GUILD_MESSAGES;

    // クライアントの構築
    let mut client = Client::builder("")
        .intents(intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // BotDataを初期化してクライアントに登録
    {
        let mut data = client.data.write().await;
        data.insert::<BotData>(serenity::prelude::Mutex::new(HashMap::new())); // serenity::prelude::Mutexを使用
    }

    // クライアントを起動
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
