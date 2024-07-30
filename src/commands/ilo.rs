use crate::{
  Error,
  internals::{
    config::BINARY_PROPERTIES,
    utils::token_path
  }
};

use reqwest::{
  ClientBuilder,
  Error as ReqError
};
use serde::{
  Serialize,
  Deserialize
};
use poise::{
  CreateReply,
  serenity_prelude::{
    CreateEmbed,
    Timestamp
  }
};

#[derive(Serialize, Deserialize)]
struct Chassis {
  #[serde(rename = "Fans")]
  fans: Vec<Fan>,
  #[serde(rename = "Temperatures")]
  temperatures: Vec<Temperature>
}

#[derive(Serialize, Deserialize)]
struct Fan {
  #[serde(rename = "CurrentReading")]
  current_reading: i32,
  #[serde(rename = "FanName")]
  fan_name: String,
  #[serde(rename = "Status")]
  status: Status,
}

#[derive(Serialize, Deserialize)]
struct Temperature {
  #[serde(rename = "CurrentReading")]
  current_reading: i32,
  #[serde(rename = "Name")]
  name: String,
  #[serde(rename = "ReadingCelsius")]
  reading_celsius: i32,
  #[serde(rename = "Status")]
  status: Status,
  #[serde(rename = "Units")]
  units: String,
  #[serde(rename = "UpperThresholdCritical")]
  upper_threshold_critical: i32,
  #[serde(rename = "UpperThresholdFatal")]
  upper_threshold_fatal: i32
}

#[derive(Serialize, Deserialize)]
struct Status {
  #[serde(rename = "Health")]
  health: Option<String>,
  #[serde(rename = "State")]
  state: String
}

async fn ilo_data() -> Result<Chassis, ReqError> {
  let client = ClientBuilder::new()
    .danger_accept_invalid_certs(true)
    .build()
    .unwrap();
  let res = client
    .get(format!("https://{}/redfish/v1/chassis/1/thermal", token_path().await.ilo_ip))
    .basic_auth(token_path().await.ilo_user, Some(token_path().await.ilo_pw))
    .send()
    .await
    .unwrap();

  let body = res.json().await.unwrap();
  Ok(body)
}

/// Retrieve data from the HP iLO4 interface
#[poise::command(
  slash_command,
  subcommands("temperature")
)]
pub async fn ilo(_: poise::Context<'_, (), Error>) -> Result<(), Error> {
  Ok(())
}

/// Retrieve data from the HP iLO4 interface
#[poise::command(slash_command)]
pub async fn temperature(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
  let data = ilo_data().await.unwrap();

  let mut tempdata = String::new();
  let mut fandata = String::new();

  let allowed_sensors = [
    "01-Inlet Ambient",
    "04-P1 DIMM 1-6",
    "13-Chipset",
    "14-Chipset Zone"
  ];

  for temp in data.temperatures {
    if temp.reading_celsius == 0 || !allowed_sensors.contains(&temp.name.as_str()) {
      continue;
    }

    let name = match temp.name.as_str() {
      "01-Inlet Ambient" => "Inlet Ambient",
      "04-P1 DIMM 1-6" => "P1 DIMM 1-6",
      "13-Chipset" => "Chipset",
      "14-Chipset Zone" => "Chipset Zone",
      _ => "Unknown Sensor"
    };

    tempdata.push_str(&format!("**{}:** `{}°C`\n", name, temp.reading_celsius));
  }
  for fan in data.fans {
    if fan.current_reading == 0 {
      continue;
    }

    fandata.push_str(&format!("**{}:** `{}%`\n", fan.fan_name, fan.current_reading));
  }

  ctx.send(CreateReply::default().embed(
    CreateEmbed::new()
      .color(BINARY_PROPERTIES.embed_color)
      .timestamp(Timestamp::now())
      .title("POMNI - HP iLO4 Temperatures")
      .fields(vec![
        ("Temperatures", tempdata, false),
        ("Fans", fandata, false)
      ])
  )).await?;

  Ok(())
}
