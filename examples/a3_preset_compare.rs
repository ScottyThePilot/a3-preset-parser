use a3_preset_parser::Preset;
use anyhow::{Error, Context};
use fs_err as fs;

use std::env::args_os;
use std::collections::HashSet;
use std::path::PathBuf;
use std::fmt;

#[derive(Debug, Clone, Copy)]
struct PresetsCompare<'p> {
  preset1: &'p Preset,
  preset1_name: &'p str,
  preset2: &'p Preset,
  preset2_name: &'p str
}

impl<'p> fmt::Display for PresetsCompare<'p> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fn fmt_list<H, L>(f: &mut fmt::Formatter<'_>, header: H, list: L) -> fmt::Result
    where H: fmt::Display, L: IntoIterator, L::Item: fmt::Display {
      let mut printed_header = false;
      for item in list {
        if !printed_header {
          writeln!(f, "{header}")?;
          printed_header = true;
        };

        writeln!(f, "- {item}")?;
      };

      if printed_header {
        writeln!(f)?;
      };

      Ok(())
    }

    let preset1_steam_mods = self.preset1.steam_mods.iter()
      .map(|steam_mod| steam_mod.id).collect::<HashSet<u64>>();
    let preset2_steam_mods = self.preset2.steam_mods.iter()
      .map(|steam_mod| steam_mod.id).collect::<HashSet<u64>>();

    if preset1_steam_mods.is_empty() && preset2_steam_mods.is_empty() {
      writeln!(f, "'{}' and '{}' have no Steam Mods\n", self.preset1_name, self.preset2_name)?;
    } else if preset1_steam_mods == preset2_steam_mods {
      writeln!(f, "'{}' and '{}' have the same Steam Mods\n", self.preset1_name, self.preset2_name)?;
    } else {
      fmt_list(f, format_args!("Steam Mods only in '{}'", self.preset1_name), {
        self.preset1.steam_mods.iter().filter(|steam_mod| !preset2_steam_mods.contains(&steam_mod.id))
      })?;

      fmt_list(f, format_args!("Steam Mods only in '{}'", self.preset2_name), {
        self.preset2.steam_mods.iter().filter(|steam_mod| !preset1_steam_mods.contains(&steam_mod.id))
      })?;

      fmt_list(f, format_args!("Steam Mods in '{}' and '{}'", self.preset1_name, self.preset2_name), {
        self.preset1.steam_mods.iter().filter(|steam_mod| preset2_steam_mods.contains(&steam_mod.id))
      })?;
    };

    let preset1_dlcs = self.preset1.dlcs.iter()
      .map(|dlc| dlc.id).collect::<HashSet<u64>>();
    let preset2_dlcs = self.preset2.dlcs.iter()
      .map(|dlc| dlc.id).collect::<HashSet<u64>>();

    if preset1_dlcs.is_empty() && preset2_dlcs.is_empty() {
      writeln!(f, "'{}' and '{}' have no DLCs\n", self.preset1_name, self.preset2_name)?;
    } else if preset1_dlcs == preset2_dlcs {
      writeln!(f, "'{}' and '{}' have the same DLCs\n", self.preset1_name, self.preset2_name)?;
    } else {
      fmt_list(f, format_args!("DLCs only in '{}'", self.preset1_name), {
        self.preset1.dlcs.iter().filter(|dlc| !preset2_dlcs.contains(&dlc.id))
      })?;

      fmt_list(f, format_args!("DLCs only in '{}'", self.preset2_name), {
        self.preset2.dlcs.iter().filter(|dlc| !preset1_dlcs.contains(&dlc.id))
      })?;

      fmt_list(f, format_args!("DLCs in '{}' and '{}'", self.preset1_name, self.preset2_name), {
        self.preset1.dlcs.iter().filter(|dlc| preset2_dlcs.contains(&dlc.id))
      })?;
    };

    fmt_list(f, format_args!("Local mods in '{}'", self.preset1_name), {
      self.preset1.local_mods.iter()
    })?;

    fmt_list(f, format_args!("Local mods in '{}'", self.preset2_name), {
      self.preset2.local_mods.iter()
    })?;

    Ok(())
  }
}

fn main() {
  run().expect("error");
}

fn run() -> Result<(), Error> {
  let mut args = args_os().skip(1).map(PathBuf::from);
  let path1 = args.next().context("expected at least 2 input files, found 0")?;
  let path2 = args.next().context("expected at least 2 input files, found 1")?;

  let document1_text = fs::read_to_string(&path1)
    .with_context(|| format!("failed to read preset file {}", path1.display()))?;
  let document2_text = fs::read_to_string(&path2)
    .with_context(|| format!("failed to read preset file {}", path2.display()))?;

  let preset1 = document1_text.parse::<Preset>()
    .with_context(|| format!("failed to parse preset file {}", path1.display()))?;
  let preset2 = document2_text.parse::<Preset>()
    .with_context(|| format!("failed to parse preset file {}", path2.display()))?;

  let mut preset1_name = preset1.preset_name.as_deref()
    .or_else(|| path1.file_stem().and_then(std::ffi::OsStr::to_str))
    .unwrap_or("Preset 1").to_owned();
  let mut preset2_name = preset2.preset_name.as_deref()
    .or_else(|| path2.file_stem().and_then(std::ffi::OsStr::to_str))
    .unwrap_or("Preset 2").to_owned();

  if preset1_name.eq_ignore_ascii_case(&preset2_name) {
    preset1_name.push_str(" (1)");
    preset2_name.push_str(" (2)");
  };

  let out = if preset1.game == preset2.game {
    if preset1.steam_mods == preset2.steam_mods && preset1.local_mods == preset2.local_mods && preset1.dlcs == preset2.dlcs {
      format!("Presets '{preset1_name}' and '{preset2_name}' have identical contents")
    } else {
      (PresetsCompare {
        preset1: &preset1,
        preset1_name: &preset1_name,
        preset2: &preset2,
        preset2_name: &preset2_name
      }).to_string()
    }
  } else {
    format!("Presets '{preset1_name}' and '{preset2_name}' do not belong to the same game")
  };

  let out_path = PathBuf::from("./out.txt");
  fs::write(&out_path, out)
    .with_context(|| format!("failed to write to {}", out_path.display()))?;

  Ok(())
}
