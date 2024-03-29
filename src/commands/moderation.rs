use crate::{
  Error,
  internals::utils::capitalize_first,
  models::moderation_events::{
    Moderations,
    ActionTypes
  }
};

use poise::CreateReply;
use poise::serenity_prelude::Member;
use parse_duration::parse;
use std::time::SystemTime;

static FALLBACK_REASON: &str = "Reason unknown";

fn duration2epoch(duration: &str) -> Result<i64, Error> {
  match parse(duration) {
    Ok(dur) => {
      let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| Error::from("System time before Unix Epoch"))?;
      Ok((now + dur).as_secs() as i64)
    }
    Err(_) => Err(Error::from("Invalid duration format"))
  }
}

/// Subcommands collection for /case command
#[poise::command(
  slash_command,
  guild_only,
  subcommands("update"),
  default_member_permissions = "KICK_MEMBERS | BAN_MEMBERS | MODERATE_MEMBERS"
)]
pub async fn case(_: poise::Context<'_, (), Error>) -> Result<(), Error> {
  Ok(())
}

/// Update a case with new reason
#[poise::command(slash_command, guild_only)]
pub async fn update(
  ctx: poise::Context<'_, (), Error>,
  #[description = "Case ID to update"] case_id: i32,
  #[description = "New reason for the case"] reason: String
) -> Result<(), Error> {
  match Moderations::update_case(
    i64::from(ctx.guild_id().unwrap()),
    case_id,
    false,
    Some(reason.clone())
  ).await {
    Ok(_) => ctx.send(CreateReply::default().content(format!("Case #{} updated with new reason:\n`{}`", case_id, reason))).await?,
    Err(e) => ctx.send(CreateReply::default().content(format!("Error updating case ID: {}\nError: {}", case_id, e))).await?
  };

  Ok(())
}

/// Kick a member
#[poise::command(
  slash_command,
  guild_only,
  default_member_permissions = "KICK_MEMBERS",
  required_bot_permissions = "KICK_MEMBERS"
)]
pub async fn kick(
  ctx: poise::Context<'_, (), Error>,
  #[description = "Member to be kicked"] member: Member,
  #[description = "Reason for the kick"] reason: Option<String>
) -> Result<(), Error> {
  let reason = reason.unwrap_or(FALLBACK_REASON.to_string());
  let case = Moderations::create_case(
    i64::from(ctx.guild_id().unwrap()),
    ActionTypes::Kick,
    false,
    i64::from(member.user.id),
    member.user.tag(),
    reason.clone(),
    i64::from(ctx.author().id),
    ctx.author().tag(),
    ctx.created_at().timestamp(),
    None
  ).await?;
  Moderations::generate_modlog(case.clone(), &ctx.http(), ctx.channel_id().into()).await?;

  member.kick_with_reason(&ctx.http(), &reason).await?;
  ctx.send(CreateReply::default().content(format!("Member: {}\nReason: `{}`\nType: {}\nModerator: {}", member.user.tag(), reason, capitalize_first(case.action_type.as_str()), ctx.author().tag()))).await?;

  Ok(())
}

/// Ban a member
#[poise::command(
  slash_command,
  guild_only,
  default_member_permissions = "BAN_MEMBERS",
  required_bot_permissions = "BAN_MEMBERS"
)]
pub async fn ban(
  ctx: poise::Context<'_, (), Error>,
  #[description = "Member to be banned"] member: Member,
  #[description = "Reason for the ban"] reason: Option<String>,
  #[description = "Ban duration"] duration: Option<String>
) -> Result<(), Error> {
  let reason = reason.unwrap_or(FALLBACK_REASON.to_string());
  let duration = match duration {
    Some(d) => Some(duration2epoch(&d)?),
    None => None
  };
  let is_case_active = duration.is_some();

  let case = Moderations::create_case(
    i64::from(ctx.guild_id().unwrap()),
    ActionTypes::Ban,
    is_case_active,
    i64::from(member.user.id),
    member.user.tag(),
    reason.clone(),
    i64::from(ctx.author().id),
    ctx.author().tag(),
    ctx.created_at().timestamp(),
    duration
  ).await?;
  Moderations::generate_modlog(case.clone(), &ctx.http(), ctx.channel_id().into()).await?;

  member.ban_with_reason(&ctx.http(), 0, &reason).await?;
  ctx.send(CreateReply::default().content(format!("Member: {}\nReason: `{}`\nType: {}\nModerator: {}\nDuration: `{}`", member.user.tag(), reason, capitalize_first(case.action_type.as_str()), ctx.author().tag(), duration.unwrap()))).await?;

  Ok(())
}

/// Timeout a member
#[poise::command(
  slash_command,
  guild_only,
  default_member_permissions = "MODERATE_MEMBERS",
  required_bot_permissions = "MODERATE_MEMBERS"
)]
pub async fn mute(
  ctx: poise::Context<'_, (), Error>,
  #[description = "Member to be muted"] mut member: Member,
  #[description = "Mute duration"] duration: String,
  #[description = "Reason for the mute"] reason: Option<String>
) -> Result<(), Error> {
  let reason = reason.unwrap_or(FALLBACK_REASON.to_string());
  let duration = Some(duration2epoch(&duration)?);
  let is_case_active = duration.is_some();

  let case = Moderations::create_case(
    i64::from(ctx.guild_id().unwrap()),
    ActionTypes::Mute,
    is_case_active,
    i64::from(member.user.id),
    member.user.tag(),
    reason.clone(),
    i64::from(ctx.author().id),
    ctx.author().tag(),
    ctx.created_at().timestamp(),
    duration
  ).await?;

  println!("case.duration: {}", case.duration.unwrap().to_string().as_str());

  let mute_time = poise::serenity_prelude::Timestamp::from_unix_timestamp(case.duration.unwrap()).expect("Failed to format timestamp");
  member.disable_communication_until_datetime(&ctx.http(), mute_time).await?;

  ctx.send(CreateReply::default().content(format!("Member: {}\nReason: `{}`\nType: {}\nModerator: {}\nDuration: `{}`", member.user.tag(), reason, capitalize_first(case.action_type.as_str()), ctx.author().tag(), mute_time))).await?;

  Ok(())
}

/// Warn a member
#[poise::command(
  slash_command,
  guild_only,
  default_member_permissions = "MODERATE_MEMBERS",
  required_bot_permissions = "MODERATE_MEMBERS"
)]
pub async fn warn(
  ctx: poise::Context<'_, (), Error>,
  #[description = "Member to be warned"] member: Member,
  #[description = "Reason for the warn"] reason: Option<String>
) -> Result<(), Error> {
  let reason = reason.unwrap_or(FALLBACK_REASON.to_string());
  let case = Moderations::create_case(
    i64::from(ctx.guild_id().unwrap()),
    ActionTypes::Warn,
    false,
    i64::from(member.user.id),
    member.user.tag(),
    reason.clone(),
    i64::from(ctx.author().id),
    ctx.author().tag(),
    ctx.created_at().timestamp(),
    None
  ).await?;
  Moderations::generate_modlog(case.clone(), &ctx.http(), ctx.channel_id().into()).await?;

  ctx.send(CreateReply::default().content(format!("Member: {}\nReason: `{}`\nType: {}\nModerator: {}", member.user.tag(), reason, capitalize_first(case.action_type.as_str()), ctx.author().tag()))).await?;

  Ok(())
}
