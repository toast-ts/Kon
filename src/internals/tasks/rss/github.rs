use crate::Error;
use super::{
  super::task_err,
  REDIS_EXPIRY_SECS,
  get_redis,
  save_to_redis,
  fetch_feed,
  parse,
  embed,
  trim_old_content,
  format_html_to_discord
};

use std::io::Cursor;
use regex::Regex;
use poise::serenity_prelude::{
  CreateEmbed,
  Timestamp
};

pub async fn github_embed() -> Result<Option<CreateEmbed>, Error> {
  let redis = get_redis().await;
  let rkey = "RSS_GitHub";
  let rkey_content = format!("{}_Content", rkey);
  let url = "https://www.githubstatus.com/history.atom";

  let res = fetch_feed(url).await?;
  let data = res.text().await?;
  let cursor = Cursor::new(data);

  let feed = parse(cursor).unwrap();
  let incident_page = feed.entries[0].links[0].clone().href;
  let article = feed.entries[0].clone();

  fn get_incident_id(input: &str) -> Option<String> {
    let re = Regex::new(r#"/incidents/([a-zA-Z0-9]+)$"#).unwrap();

    if let Some(caps) = re.captures(input) {
      Some(caps[1].to_string())
    } else {
      None
    }
  }

  let cached_incident = redis.get(&rkey).await.unwrap().unwrap_or_default();
  let new_content = format_html_to_discord(article.content.unwrap().body.unwrap());

  let color: u32;
  let update_patt = Regex::new(r"(?i)\bupdate\b").unwrap();
  let resolved_patt = Regex::new(r"(?i)\bresolved\b").unwrap();
  let date_patt = Regex::new(r"\b[A-Z][a-z]{2} \d{2}, \d{2}:\d{2} UTC\b").unwrap();

  let first_entry = date_patt.split(&new_content).next().unwrap_or(&new_content);

  if update_patt.is_match(&first_entry) {
    color = 0xFFAD33;
  } else if resolved_patt.is_match(&first_entry) {
    color = 0x57F287;
  } else {
    color = 0x243C32;
  }

  if cached_incident.is_empty() {
    redis.set(&rkey, &get_incident_id(&article.links[0].href).unwrap()).await.unwrap();
    redis.set(&rkey_content, &new_content).await.unwrap();
    if let Err(y) = redis.expire(&rkey, REDIS_EXPIRY_SECS).await {
      task_err("RSS", format!("[RedisExpiry]: {}", y).as_str());
    }
    return Ok(None);
  }

  if let Some(incident) = get_incident_id(&article.links[0].href) {
    if incident == cached_incident {
      let cached_content: String = redis.get(&rkey_content).await.unwrap().unwrap_or_default();
      if cached_content == new_content {
        return Ok(None);
      } else {
        redis.set(&rkey_content, &new_content).await.unwrap();
        redis.expire(&rkey_content, 21600).await.unwrap();
        return Ok(Some(embed(
          color,
          article.title.unwrap().content,
          incident_page,
          trim_old_content(&new_content),
          Timestamp::from(article.updated.unwrap())
        )));
      }
    } else {
      save_to_redis(&rkey, &incident).await?;
      redis.set(&rkey_content, &new_content).await.unwrap();
      return Ok(Some(embed(
        color,
        article.title.unwrap().content,
        incident_page,
        trim_old_content(&new_content),
        Timestamp::from(article.updated.unwrap())
      )));
    }
  } else {
    task_err("RSS:GitHub", &format!("Incident ID does not match the expected RegEx pattern! ({})", &article.links[0].href));
    Ok(None)
  }
}
