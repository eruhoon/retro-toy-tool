use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename = "gameList")]
pub struct GameListXml {
    #[serde(rename = "game", default)]
    pub games: Vec<GameXml>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameXml {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub video: Option<String>,
    #[serde(default)]
    pub rating: Option<f32>,
    #[serde(default)]
    pub releasedate: Option<String>,
    #[serde(default)]
    pub developer: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub players: Option<String>,
    #[serde(default)]
    pub favorite: Option<bool>,
    #[serde(default)]
    pub hidden: Option<bool>,
}

pub fn parse_gamelist_xml(xml_content: &str) -> Result<GameListXml, String> {
    if xml_content.trim().is_empty() {
        return Ok(GameListXml::default());
    }
    quick_xml::de::from_str(xml_content)
        .map_err(|e| format!("gamelist.xml 파싱 오류: {}", e))
}

pub fn serialize_gamelist_xml(gamelist: &GameListXml) -> Result<String, String> {
    let mut xml = String::from("<?xml version=\"1.0\"?>\n<gameList>\n");
    for game in &gamelist.games {
        xml.push_str("  <game>\n");
        xml.push_str(&format!("    <path>{}</path>\n", escape_xml(&game.path)));
        if let Some(ref name) = game.name {
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(name)));
        }
        if let Some(ref desc) = game.desc {
            xml.push_str(&format!("    <desc>{}</desc>\n", escape_xml(desc)));
        }
        if let Some(ref image) = game.image {
            xml.push_str(&format!("    <image>{}</image>\n", escape_xml(image)));
        }
        if let Some(ref thumb) = game.thumbnail {
            xml.push_str(&format!("    <thumbnail>{}</thumbnail>\n", escape_xml(thumb)));
        }
        if let Some(ref video) = game.video {
            xml.push_str(&format!("    <video>{}</video>\n", escape_xml(video)));
        }
        if let Some(rating) = game.rating {
            xml.push_str(&format!("    <rating>{:.2}</rating>\n", rating));
        }
        if let Some(ref rel) = game.releasedate {
            xml.push_str(&format!("    <releasedate>{}</releasedate>\n", escape_xml(rel)));
        }
        if let Some(ref dev) = game.developer {
            xml.push_str(&format!("    <developer>{}</developer>\n", escape_xml(dev)));
        }
        if let Some(ref publ) = game.publisher {
            xml.push_str(&format!("    <publisher>{}</publisher>\n", escape_xml(publ)));
        }
        if let Some(ref genre) = game.genre {
            xml.push_str(&format!("    <genre>{}</genre>\n", escape_xml(genre)));
        }
        if let Some(ref players) = game.players {
            xml.push_str(&format!("    <players>{}</players>\n", escape_xml(players)));
        }
        if let Some(fav) = game.favorite {
            xml.push_str(&format!("    <favorite>{}</favorite>\n", fav));
        }
        if let Some(hid) = game.hidden {
            xml.push_str(&format!("    <hidden>{}</hidden>\n", hid));
        }
        xml.push_str("  </game>\n");
    }
    xml.push_str("</gameList>\n");
    Ok(xml)
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// Normalizes path for matching by filename (e.g., "./Super Mario.zip" -> "Super Mario.zip")
pub fn normalize_rom_path(path: &str) -> String {
    let p = path.trim();
    let p = p.strip_prefix("./").unwrap_or(p);
    let p = p.rsplit('/').next().unwrap_or(p);
    let p = p.rsplit('\\').next().unwrap_or(p);
    p.to_lowercase()
}

// Normalizes relative path while PRESERVING subdirectory structure
// e.g. "./ST3739190/game.sh" -> "st3739190/game.sh"
// e.g. "ports/ST3739190/game.sh" (system "ports") -> "st3739190/game.sh"
pub fn normalize_rel_path(path: &str, system_id: &str) -> String {
    let p = path.trim().replace('\\', "/");
    let p = p.strip_prefix("./").unwrap_or(&p);
    let p = p.trim_start_matches('/');
    let lower = p.to_lowercase();

    let sys_prefix = format!("{}/", system_id.to_lowercase());
    if let Some(stripped) = lower.strip_prefix(&sys_prefix) {
        stripped.to_string()
    } else {
        lower
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_serialize() {
        let sample_xml = r#"<?xml version="1.0"?>
<gameList>
  <game>
    <path>./Super Mario World (USA).zip</path>
    <name>Super Mario World</name>
    <desc>A classic 16-bit adventure.</desc>
    <image>./images/Super Mario World (USA)-image.png</image>
    <rating>0.95</rating>
    <releasedate>19901121T000000</releasedate>
    <developer>Nintendo</developer>
    <publisher>Nintendo</publisher>
    <genre>Platform</genre>
    <players>1-2</players>
    <favorite>true</favorite>
  </game>
</gameList>"#;

        let parsed = parse_gamelist_xml(sample_xml).expect("Should parse xml");
        assert_eq!(parsed.games.len(), 1);
        let g = &parsed.games[0];
        assert_eq!(g.name.as_deref(), Some("Super Mario World"));
        assert_eq!(g.favorite, Some(true));
        assert_eq!(g.rating, Some(0.95));

        let reserialized = serialize_gamelist_xml(&parsed).expect("Should serialize");
        assert!(reserialized.contains("<name>Super Mario World</name>"));
        assert!(reserialized.contains("<favorite>true</favorite>"));
    }

    #[test]
    fn test_normalize_rom_path() {
        assert_eq!(normalize_rom_path("./snes/smw.sfc"), "smw.sfc");
        assert_eq!(normalize_rom_path("SMW.SFC"), "smw.sfc");
        assert_eq!(normalize_rom_path(".\\smw.sfc"), "smw.sfc");
    }
}
