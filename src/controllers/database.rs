use crate::internals;

use poise::serenity_prelude::prelude::TypeMapKey;
use tokio_postgres::{Client, NoTls, Error};

pub struct DatabaseController {
  pub client: Client
}

impl TypeMapKey for DatabaseController {
  type Value = DatabaseController;
}

impl DatabaseController {
  pub async fn new() -> Result<DatabaseController, Error> {
    let (client, connection) = tokio_postgres::connect(&internals::utils::token_path().await.postgres_uri, NoTls).await?;

    tokio::spawn(async move {
      if let Err(e) = connection.await {
        eprintln!("Connection error: {}", e);
      }
    });

    // Gameservers
    client.batch_execute("
      CREATE TABLE IF NOT EXISTS gameservers (
        server_name VARCHAR(255) NOT NULL,
        game_name VARCHAR(255) NOT NULL,
        guild_owner BIGINT NOT NULL,
        ip_address VARCHAR(255) NOT NULL,
        PRIMARY KEY (server_name, guild_owner)
      );
    ").await?;

    // Guild Case IDs
    client.batch_execute("
      CREATE TABLE IF NOT EXISTS guild_case_ids (
        guild_id BIGINT NOT NULL,
        max_case_id INT NOT NULL DEFAULT 0,
        PRIMARY KEY (guild_id)
      );
    ").await?;

    // ModerationEvents
    client.batch_execute("
      CREATE TABLE IF NOT EXISTS moderation_events (
        guild_id BIGINT NOT NULL,
        case_id INT NOT NULL,
        action_type VARCHAR(255) NOT NULL,
        is_active BOOLEAN NOT NULL DEFAULT FALSE,
        user_id BIGINT NOT NULL,
        user_tag VARCHAR(255) NOT NULL,
        reason VARCHAR(1024) NOT NULL,
        moderator_id BIGINT NOT NULL,
        moderator_tag VARCHAR(255) NOT NULL,
        time_created BIGINT NOT NULL,
        duration BIGINT,
        PRIMARY KEY (guild_id, case_id)
      );
    ").await?;

    Ok(DatabaseController { client })
  }
}
