use {
  kon_libs::{
    KonError,
    KonResult,
    format_bytes,
    mention_dev
  },
  poise::{
    CreateReply,
    serenity_prelude::{
      CreateAttachment,
      Message
    }
  },
  regex::Regex,
  std::{
    fs::{
      metadata,
      remove_file,
      write
    },
    os::unix::fs::MetadataExt
  }
};

/// Convert MIDI file to WAV
#[poise::command(
  context_menu_command = "MIDI -> WAV",
  install_context = "Guild|User",
  interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn midi_to_wav(
  ctx: super::PoiseCtx<'_>,
  #[description = "MIDI file to be converted"] message: Message
) -> KonResult<()> {
  let re = Regex::new(r"(?i)\.mid$").unwrap();

  if !message.embeds.is_empty() || message.attachments.is_empty() || !re.is_match(&message.attachments[0].filename) {
    ctx.reply("That ain't a MIDI file! What are you even doing??").await?;
    return Ok(());
  }

  ctx.defer().await?;

  let bytes = match message.attachments[0].download().await {
    Ok(bytes) => bytes,
    Err(y) => {
      ctx
        .send(CreateReply::default().content(format!(
          "Download failed, ask {} to check console for more information!",
          mention_dev(ctx).unwrap_or_default()
        )))
        .await
        .unwrap();

      return Err(KonError::from(format!("Failed to download the file: {y}")))
    }
  };

  let midi_path = &message.attachments[0].filename;
  write(midi_path, bytes)?;

  let wav_path = re.replace(midi_path, ".wav");

  let sf2_path = "/tmp/FluidR3_GM.sf2";
  write(sf2_path, include_bytes!("../../../libs/assets/FluidR3_GM.sf2"))?;

  let output = std::process::Command::new("fluidsynth")
    .args(["-ni", sf2_path, midi_path, "-F", &wav_path])
    .output();

  // Just to add an info to console to tell what the bot is doing when MIDI file is downloaded.
  println!("Discord[{}]: Processing MIDI file: \"{midi_path}\"", ctx.command().qualified_name);

  match output {
    Ok(_) => {
      let reply = ctx
        .send(CreateReply::default().attachment(CreateAttachment::path(&*wav_path).await.unwrap()))
        .await;

      if reply.is_err() {
        println!(
          "Discord[{}]: Processed file couldn't be uploaded back to Discord channel due to upload limit",
          ctx.command().qualified_name
        );

        ctx
          .send(CreateReply::default().content(format!(
            "Couldn't upload the processed file (`{}`, `{}`) due to upload limit",
            &*wav_path,
            format_bytes(metadata(&*wav_path).unwrap().size())
          )))
          .await?;
      } else if reply.is_ok() {
        println!(
          "Discord[{}]: Processed file uploaded back to Discord channel",
          ctx.command().qualified_name
        );
        remove_file(midi_path)?;
        remove_file(&*wav_path)?;
      }
    },
    Err(y) => {
      ctx
        .send(CreateReply::default().content("Command didn't execute successfully, check console for more information!"))
        .await?;

      return Err(KonError::from(format!("Midi conversion failed: {y}")))
    }
  }

  Ok(())
}
