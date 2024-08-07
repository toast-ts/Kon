use crate::{
  Error,
  GIT_COMMIT_HASH,
  internals::utils::{
    BOT_VERSION,
    format_duration,
    concat_message
  }
};

use sysinfo::System;
use uptime_lib::get;
use std::{
  fs::File,
  path::Path,
  time::{
    Duration,
    SystemTime,
    UNIX_EPOCH
  },
  io::{
    BufRead,
    BufReader
  }
};

fn get_os_info() -> String {
  let path = Path::new("/etc/os-release");
  let mut name = "BoringOS".to_string();
  let mut version = "v0.0".to_string();

  if let Ok(file) = File::open(&path) {
    let reader = BufReader::new(file);
    for line in reader.lines() {
      if let Ok(line) = line {
        if line.starts_with("NAME=") {
          name = line.split('=').nth(1).unwrap_or_default().trim_matches('"').to_string();
        } else if line.starts_with("VERSION=") {
          version = line.split('=').nth(1).unwrap_or_default().trim_matches('"').to_string();
        } else if line.starts_with("VERSION_ID=") {
          version = line.split('=').nth(1).unwrap_or_default().trim_matches('"').to_string();
        }
      }
    }
  }

  format!("{} {}", name, version)
}

/// Retrieve host and bot uptimes
#[poise::command(slash_command)]
pub async fn uptime(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
  let _bot = ctx.http().get_current_user().await.unwrap();
  let mut sys = System::new_all();
  sys.refresh_all();

  // Fetch system's uptime
  let sys_uptime = get().unwrap().as_secs();

  // Fetch system's processor
  let cpu = sys.cpus();

  // Fetch bot's process uptime
  let curr_pid = sysinfo::get_current_pid().unwrap();
  let now = SystemTime::now();
  let mut proc_uptime = 0;
  if let Some(process) = sys.process(curr_pid) {
    let time_started = UNIX_EPOCH + Duration::from_secs(process.start_time());
    proc_uptime = now.duration_since(time_started).unwrap().as_secs();
  }

  let stat_msg = vec![
    format!("**{} {}** `{}`", _bot.name, BOT_VERSION.as_str(), GIT_COMMIT_HASH),
    format!(">>> System: `{}`", format_duration(sys_uptime)),
    format!("Process: `{}`", format_duration(proc_uptime)),
    format!("CPU: `{}`", format!("{}", cpu[0].brand())),
    format!("OS: `{}`", get_os_info())
  ];
  ctx.reply(concat_message(stat_msg)).await?;

  Ok(())
}
