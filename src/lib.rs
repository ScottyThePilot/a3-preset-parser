use scraper::{ElementRef, Html, Selector};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

macro_rules! lazy_selector {
  ($selector:literal) => (LazyLock::new(|| Selector::parse($selector).unwrap()));
}

macro_rules! lazy_selectors {
  ($($vis:vis static $SELECTOR_NAME:ident = $selector:literal;)*) => ($(
    $vis static $SELECTOR_NAME: LazyLock<Selector> = lazy_selector!($selector);
  )*);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum Game {
  Arma,
  DayZ
}

impl fmt::Display for Game {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match self {
      Game::Arma => "Arma 3",
      Game::DayZ => "DayZ"
    })
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct PresetSteamMod {
  pub display_name: String,
  pub id: u64
}

impl fmt::Display for PresetSteamMod {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "https://{STEAM_WORKSHOP_LINK}{}: {}", self.id, self.display_name)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct PresetLocalMod {
  pub display_name: String
}

impl fmt::Display for PresetLocalMod {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.display_name)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct PresetDlc {
  pub display_name: String,
  pub id: u64
}

impl fmt::Display for PresetDlc {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "https://{STEAM_APP_LINK}{}: {}", self.id, self.display_name)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Preset {
  pub game: Game,
  pub preset_name: Option<String>,
  pub steam_mods: Vec<PresetSteamMod>,
  pub local_mods: Vec<PresetLocalMod>,
  pub dlcs: Vec<PresetDlc>
}

impl fmt::Display for Preset {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if let Some(preset_name) = self.preset_name.as_deref() {
      writeln!(f, "{} Preset: {preset_name}", self.game)?;
    } else {
      writeln!(f, "{} Preset", self.game)?;
    };

    for m in self.steam_mods.iter() {
      writeln!(f, "Steam: {m}")?;
    };

    for m in self.local_mods.iter() {
      writeln!(f, "Local: {m}")?;
    };

    for m in self.dlcs.iter() {
      writeln!(f, "DLC: {m}")?;
    };

    Ok(())
  }
}

#[derive(Debug, Error)]
pub enum Error {
  #[error("preset type selector failed on html: {0}")]
  SelectorFailedPresetType(String),
  #[error("invalid preset type value {0:?}, expected one of 'preset' or 'list'")]
  InvalidPresetTypeValue(String),
  #[error("item origin selector failed on html: {0}")]
  SelectorFailedItemOrigin(String),
  #[error("invalid item origin value {0:?}, expected one of 'from-local' or 'from-steam'")]
  InvalidItemOriginValue(String),
  #[error("item name selector failed on html: {0}")]
  SelectorFailedItemName(String),
  #[error("item link selector failed on html: {0}")]
  SelectorFailedItemLink(String),
  #[error("invalid item link value {0:?}, failed to extract steam workshop item id")]
  InvalidItemLinkSteamWorkshop(String),
  #[error("invalid item link value {0:?}, failed to extract steam app item id")]
  InvalidItemLinkSteamApp(String)
}

impl FromStr for Preset {
  type Err = Error;

  fn from_str(document_text: &str) -> Result<Self, Self::Err> {
    lazy_selectors!{
      static SELECTOR_PRESET_TYPE_ARMA = "head > meta[name=\"arma:Type\"][content]";
      static SELECTOR_PRESET_NAME_ARMA = "head > meta[name=\"arma:PresetName\"][content]";
      static SELECTOR_PRESET_TYPE_DAYZ = "head > meta[name=\"dayz:Type\"][content]";
      static SELECTOR_PRESET_NAME_DAYZ = "head > meta[name=\"dayz:PresetName\"][content]";
      static SELECTOR_MOD_CONTAINER = "body > div.mod-list > table tr[data-type=\"ModContainer\"]";
      static SELECTOR_DLC_CONTAINER = "body > div.dlc-list > table tr[data-type=\"DlcContainer\"]";
      static SELECTOR_ITEM_NAME = "td[data-type=\"DisplayName\"]";
      static SELECTOR_ITEM_LINK = "td > a[data-type=\"Link\"]";
      static SELECTOR_ITEM_ORIGIN = "td > span[class]";
    }

    fn select_preset_type(document: &Html) -> Result<Game, Error> {
      let [arma, dayz] = [
        (&SELECTOR_PRESET_TYPE_ARMA, Game::Arma),
        (&SELECTOR_PRESET_TYPE_DAYZ, Game::DayZ)
      ].map(|(selector, game)| {
        document.select(selector).next()
          .and_then(|element| element.value().attr("content"))
          .ok_or_else(|| Error::SelectorFailedPresetType(document.html()))
          .and_then(|content| if ["list", "preset"].contains(&content) {
            Ok(game)
          } else {
            Err(Error::InvalidPresetTypeValue(content.to_owned()))
          })
      });

      Result::or(arma, dayz)
    }

    fn select_preset_name_arma(document: &Html) -> Option<&str> {
      document.select(&SELECTOR_PRESET_NAME_ARMA).next()
        .and_then(|element| element.value().attr("content"))
    }

    fn select_preset_name_dayz(document: &Html) -> Option<&str> {
      document.select(&SELECTOR_PRESET_NAME_DAYZ).next()
        .and_then(|element| element.value().attr("content"))
    }

    fn select_item_name(element: ElementRef<'_>) -> Result<&str, Error> {
      element.select(&SELECTOR_ITEM_NAME).next()
        .and_then(|element| element.text().next())
        .ok_or_else(|| Error::SelectorFailedItemName(element.inner_html()))
    }

    fn select_item_link(element: ElementRef<'_>) -> Result<&str, Error> {
      element.select(&SELECTOR_ITEM_LINK).next()
        .and_then(|element| element.value().attr("href"))
        .ok_or_else(|| Error::SelectorFailedItemLink(element.inner_html()))
    }

    fn select_item_origin(element: ElementRef<'_>) -> Result<&str, Error> {
      element.select(&SELECTOR_ITEM_ORIGIN).next()
        .and_then(|element| element.value().attr("class"))
        .ok_or_else(|| Error::SelectorFailedItemOrigin(element.inner_html()))
    }

    let document = Html::parse_document(&document_text);

    let game = select_preset_type(&document)?;

    let preset_name = match game {
      Game::Arma => select_preset_name_arma(&document),
      Game::DayZ => select_preset_name_dayz(&document),
    };

    let mut steam_mods = Vec::new();
    let mut local_mods = Vec::new();
    for mod_element in document.select(&SELECTOR_MOD_CONTAINER) {
      let display_name = select_item_name(mod_element)?;

      match select_item_origin(mod_element)? {
        "from-local" => {
          local_mods.push(PresetLocalMod { display_name: display_name.to_owned() });
        },
        "from-steam" => {
          let link = select_item_link(mod_element)?;
          let id = get_steam_link_steam_workshop_id(link)
            .ok_or_else(|| Error::InvalidItemLinkSteamWorkshop(link.to_owned()))?;
          steam_mods.push(PresetSteamMod { display_name: display_name.to_owned(), id });
        },
        origin => {
          return Err(Error::InvalidItemOriginValue(origin.to_owned()));
        }
      };
    };

    let mut dlcs = Vec::new();
    for dlc_element in document.select(&SELECTOR_DLC_CONTAINER) {
      let display_name = select_item_name(dlc_element)?;
      let link = select_item_link(dlc_element)?;
      let id = get_steam_link_steam_app_id(link)
        .ok_or_else(|| Error::InvalidItemLinkSteamApp(link.to_owned()))?;
      dlcs.push(PresetDlc { display_name: display_name.to_owned(), id });
    };

    Ok(Preset {
      game,
      preset_name: preset_name.map(str::to_owned),
      steam_mods,
      local_mods,
      dlcs
    })
  }
}

const STEAM_WORKSHOP_LINK: &str = "steamcommunity.com/sharedfiles/filedetails/?id=";
const STEAM_APP_LINK: &str = "store.steampowered.com/app/";

fn get_steam_link_steam_workshop_id(link: &str) -> Option<u64> {
  strip_url_protocol(link)
    .and_then(|link| link.strip_prefix(STEAM_WORKSHOP_LINK))
    .and_then(|id| id.parse::<u64>().ok())
}

fn get_steam_link_steam_app_id(link: &str) -> Option<u64> {
  strip_url_protocol(link)
    .and_then(|link| link.strip_prefix(STEAM_APP_LINK))
    .and_then(|id| id.parse::<u64>().ok())
}

fn strip_url_protocol(link: &str) -> Option<&str> {
  let link = link.trim();
  Option::or(
    link.strip_prefix("https://"),
    link.strip_prefix("http://")
  )
}
