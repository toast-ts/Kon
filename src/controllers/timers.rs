use crate::{
  Error,
  models::moderation_events::{
    Moderations,
    ActionTypes
  }
};

use std::time::SystemTime;
use poise::serenity_prelude::{
  Context,
  model::{
    user::CurrentUser,
    id::{
      UserId,
      GuildId
    }
  },
};
use tokio::time::{
  interval,
  Duration
};

fn timer_failed(name: &str) -> String {
  format!("Failed to start timer for {}", name)
}

pub async fn start_timers(discord_: &Context, bot_: CurrentUser) -> Result<(), Error> {
  let ctx_clone = discord_.clone();
  tokio::spawn(async move {
    check_modlog_cases(&ctx_clone, bot_).await.expect(&timer_failed("moderation events"))
  });

  Ok(())
}

async fn check_modlog_cases(discord_: &Context, bot_: CurrentUser) -> Result<(), Error> {
  let mut interval = interval(Duration::from_secs(6));

  loop {
    interval.tick().await;
    let events = Moderations::get_active_events().await?;

    for event in events {
      let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| Error::from("System time before Unix Epoch"))?;

      let check_action_type = match event.action_type {
        ActionTypes::Ban => ActionTypes::Unban,
        ActionTypes::Mute => ActionTypes::Unmute,
        _ => continue // Skip if not a timed action
      };

      if let Some(duration) = event.duration {
        let duration = Duration::from_secs(duration as u64);
        if now > duration {
          Moderations::update_case(
            event.guild_id,
            event.case_id,
            false,
            None
          ).await?;
          Moderations::generate_modlog(Moderations::create_case(
            event.guild_id,
            check_action_type,
            false,
            event.user_id,
            event.user_tag.clone(),
            format!("Duration for Case #{} has expired", event.case_id),
            bot_.id.into(),
            bot_.tag(),
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
            None
          ).await?, &discord_.http, 865673694184996888).await?;

          match check_action_type {
            ActionTypes::Unban => {
              let guild_id = GuildId::new(event.guild_id as u64);
              let user_id = UserId::new(event.user_id as u64);
              discord_.http.remove_ban(guild_id, user_id, Some(format!("Duration for Case #{} has expired", event.case_id).as_str())).await?;
            },
            _ => {}
          }

          let guild_id = GuildId::new(event.guild_id as u64);
          let cached_guild_data = discord_.cache.guild(guild_id);

          println!("ModerationTimer[CaseExpired]: {}:#{}:{}:{}", cached_guild_data.unwrap().name.to_owned(), event.case_id, event.user_tag, event.reason)
        }
      }
    }
  }
}
