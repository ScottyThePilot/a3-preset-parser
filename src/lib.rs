use scraper::{ElementRef, Html, Selector};
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
pub enum Game {
  Arma,
  DayZ
}

impl Game {
  fn from_attr_value(s: &str) -> Option<Self> {
    match s {
      "arma:Type" => Some(Self::Arma),
      "dayz:Type" => Some(Self::DayZ),
      _ => None
    }
  }
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
pub struct PresetLocalMod {
  pub display_name: String
}

impl fmt::Display for PresetLocalMod {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.display_name)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
pub struct Preset {
  pub game: Game,
  pub steam_mods: Vec<PresetSteamMod>,
  pub local_mods: Vec<PresetLocalMod>,
  pub dlc: Vec<PresetDlc>
}

impl fmt::Display for Preset {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    writeln!(f, "{} Preset", self.game)?;
    for m in self.steam_mods.iter() {
      writeln!(f, "Steam: {m}")?;
    };
    for m in self.local_mods.iter() {
      writeln!(f, "Local: {m}")?;
    };
    for m in self.dlc.iter() {
      writeln!(f, "DLC: {m}")?;
    };

    Ok(())
  }
}

#[derive(Debug, Error)]
pub enum Error {
  #[error("preset type selector failed on html: {0}")]
  SelectorFailedPresetType(String),
  #[error("invalid preset type value {0:?}, expected one of 'arma:Type' or 'dayz:Type'")]
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
      static SELECTOR_PRESET_TYPE = "head > meta[name]";
      static SELECTOR_MOD_CONTAINER = "body > div.mod-list > table tr[data-type=\"ModContainer\"]";
      static SELECTOR_DLC_CONTAINER = "body > div.dlc-list > table tr[data-type=\"DlcContainer\"]";
      static SELECTOR_ITEM_NAME = "td[data-type=\"DisplayName\"]";
      static SELECTOR_ITEM_LINK = "td > a[data-type=\"Link\"]";
      static SELECTOR_ITEM_ORIGIN = "td > span[class]";
    }

    fn select_preset_type(document: &Html) -> Result<&str, Error> {
      document.select(&SELECTOR_PRESET_TYPE).next()
        .and_then(|element| element.value().attr("name"))
        .ok_or_else(|| Error::SelectorFailedPresetType(document.html()))
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

    let preset_type = select_preset_type(&document)?;
    let game = Game::from_attr_value(preset_type)
      .ok_or_else(|| Error::InvalidPresetTypeValue(preset_type.to_owned()))?;

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

    let mut dlc = Vec::new();
    for dlc_element in document.select(&SELECTOR_DLC_CONTAINER) {
      let display_name = select_item_name(dlc_element)?;
      let link = select_item_link(dlc_element)?;
      let id = get_steam_link_steam_app_id(link)
        .ok_or_else(|| Error::InvalidItemLinkSteamApp(link.to_owned()))?;
      dlc.push(PresetDlc { display_name: display_name.to_owned(), id });
    };

    Ok(Preset { game, steam_mods, local_mods, dlc })
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
