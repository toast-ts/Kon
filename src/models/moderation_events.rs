use crate::{
  controllers::database::DatabaseController,
  internals::utils::{
    EMBED_COLOR,
    capitalize_first
  }
};

use poise::serenity_prelude::{
  Http,
  Error,
  Timestamp,
  ChannelId,
  CreateMessage,
  CreateEmbed
};

#[derive(Clone)]
pub struct Moderations {
  pub guild_id: i64,
  pub case_id: i32,
  pub action_type: ActionTypes,
  pub is_active: bool,
  pub user_id: i64,
  pub user_tag: String,
  pub reason: String,
  pub moderator_id: i64,
  pub moderator_tag: String,
  pub time_created: i64,
  pub duration: Option<i64>
}

#[derive(Clone, Copy)]
pub enum ActionTypes {
  Ban,
  Kick,
  Mute,
  Warn,
  Unban,
  Unmute,
  Unknown
}

impl ActionTypes {
  pub fn as_str(&self) -> &'static str {
    match *self {
      ActionTypes::Ban => "ban",
      ActionTypes::Kick => "kick",
      ActionTypes::Mute => "mute",
      ActionTypes::Warn => "warn",
      ActionTypes::Unban => "unban",
      ActionTypes::Unmute => "unmute",
      ActionTypes::Unknown => "unknown"
    }
  }
}

impl Moderations {
  pub async fn get_active_events() -> Result<Vec<Moderations>, tokio_postgres::Error> {
    let _db = DatabaseController::new().await?.client;

    _db.execute("BEGIN", &[]).await.expect("Failed to start transaction!");
    let stmt = _db.prepare("
      SELECT * FROM moderation_events
      WHERE is_active = true
      ORDER BY duration DESC, time_created DESC;
    ").await?;

    _db.execute("COMMIT", &[]).await.expect("Failed to commit transaction!");
    let rows = _db.query(&stmt, &[]).await?;

    let mut moderations = Vec::new();
    for row in rows {
      moderations.push(Moderations {
        guild_id: row.get("guild_id"),
        case_id: row.get("case_id"),
        action_type: match row.get::<_, &str>("action_type") {
          "ban" => ActionTypes::Ban,
          "kick" => ActionTypes::Kick,
          "mute" => ActionTypes::Mute,
          "warn" => ActionTypes::Warn,
          "unban" => ActionTypes::Unban,
          "unmute" => ActionTypes::Unmute,
          _ => ActionTypes::Unknown
        },
        is_active: row.get("is_active"),
        user_id: row.get("user_id"),
        user_tag: row.get("user_tag"),
        reason: row.get("reason"),
        moderator_id: row.get("moderator_id"),
        moderator_tag: row.get("moderator_tag"),
        time_created: row.get("time_created"),
        duration: row.get("duration")
      });
    }

    Ok(moderations)
  }

  pub async fn generate_modlog(case: Moderations, http: &Http, channel_id: u64) -> Result<(), Error> {
    let time_created_formatted = Timestamp::from_unix_timestamp(case.time_created).expect(" Failed to format timestamp!");
    let modlog_embed = CreateEmbed::default()
      .color(EMBED_COLOR)
      .title(format!("{} • Case #{}", capitalize_first(case.action_type.as_str()), case.case_id))
      .fields(vec![
        ("User", format!("{}\n<@{}>", case.user_tag, case.user_id), true),
        ("Moderator", format!("{}\n<@{}>", case.moderator_tag, case.moderator_id), true),
        ("\u{200B}", "\u{200B}".to_string(), true),
        ("Reason", format!("`{}`", case.reason), false)
      ])
      .timestamp(time_created_formatted);

    ChannelId::new(channel_id).send_message(http, CreateMessage::new().embed(modlog_embed)).await?;

    Ok(())
  }

  pub async fn create_case(
    guild_id: i64,
    action_type: ActionTypes,
    is_active: bool,
    user_id: i64,
    user_tag: String,
    reason: String,
    moderator_id: i64,
    moderator_tag: String,
    time_created: i64,
    duration: Option<i64>
  ) -> Result<Moderations, tokio_postgres::Error> {
    let _db = DatabaseController::new().await?.client;

    // Get the current max case_id for the guild
    let stmt = _db.prepare("
      SELECT max_case_id FROM guild_case_ids WHERE guild_id = $1;
    ").await?;
    let rows = _db.query(&stmt, &[&guild_id]).await?;
    let mut max_case_id = if let Some(row) = rows.get(0) {
      row.get::<_, i32>("max_case_id")
    } else {
      0
    };

    // Increment the max case_id for the guild
    max_case_id += 1;
    let stmt = _db.prepare("
      INSERT INTO guild_case_ids (guild_id, max_case_id) VALUES ($1, $2)
      ON CONFLICT (guild_id) DO UPDATE SET max_case_id = $2;
    ").await?;
    _db.execute(&stmt, &[&guild_id, &max_case_id]).await?;

    // Create a new case in database and return the case_id
    let stmt = _db.prepare("
      INSERT INTO moderation_events (
        guild_id, case_id, action_type, is_active, user_id, user_tag, reason, moderator_id, moderator_tag, time_created, duration
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
      RETURNING case_id;
    ").await?;

    let row = _db.query(&stmt, &[
      &guild_id,
      &max_case_id,
      &action_type.as_str(),
      &is_active,
      &user_id,
      &user_tag,
      &reason,
      &moderator_id,
      &moderator_tag,
      &time_created,
      &duration
    ]).await?;

    let moderations = Moderations {
      guild_id,
      case_id: row[0].get("case_id"),
      action_type,
      is_active,
      user_id,
      user_tag,
      reason,
      moderator_id,
      moderator_tag,
      time_created,
      duration
    };

    Ok(moderations)
  }

  pub async fn update_case(
    guild_id: i64,
    case_id: i32,
    is_active: bool,
    reason: Option<String>
  ) -> Result<(), tokio_postgres::Error> {
    let _db = DatabaseController::new().await?.client;

    match reason {
      Some(reason) => {
        _db.execute("
          UPDATE moderation_events
          SET is_active = $3, reason = $4 WHERE guild_id = $1 AND case_id = $2;
        ", &[&guild_id, &case_id, &is_active, &reason]).await?;
      },
      None => {
        _db.execute("
          UPDATE moderation_events
          SET is_active = $3 WHERE guild_id = $1 AND case_id = $2;
        ", &[&guild_id, &case_id, &is_active]).await?;
      }
    }

    Ok(())
  }
}
