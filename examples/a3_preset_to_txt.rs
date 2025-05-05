use a3_preset_parser::Preset;
use anyhow::{Error, Context};
use fs_err as fs;

use std::env::args_os;
use std::path::PathBuf;

fn main() {
  run().expect("error");
}

fn run() -> Result<(), Error> {
  for path in args_os().skip(1).map(PathBuf::from) {
    let document_text = fs::read_to_string(&path)
      .with_context(|| format!("failed to read preset file {}", path.display()))?;
    let preset = document_text.parse::<Preset>()
      .with_context(|| format!("failed to parse preset file {}", path.display()))?;
    let out_path = path.with_extension("txt");
    fs::write(&out_path, preset.to_string())
      .with_context(|| format!("failed to write to {}", out_path.display()))?;
  };

  Ok(())
}
