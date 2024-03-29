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

    Ok(DatabaseController { client })
  }
}
