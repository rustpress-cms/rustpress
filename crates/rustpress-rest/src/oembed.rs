//! `oembed/1.0/embed`: the oEmbed provider of the site (`WP_oEmbed_Controller`).

use rustpress_blocks::content::the_title_for;
use rustpress_blocks::images;
use serde_json::{Map, Value};

use crate::error::RestError;
use crate::params::Params;
use crate::{Outcome, Resp, Rest, Result};

/// `_oembed_create_xml()`: values go through `esc_html()` and then SimpleXML, which decodes the
/// entities and escapes the text again.
fn xml_text(value: &str) -> String {
    let escaped = rustpress_core::formatting::esc_html(value);
    let decoded = rustpress_core::formatting::html_entity_decode(&escaped).replace("&#039;", "'");
    // The document has no encoding, so libxml writes everything outside ASCII as a character reference.
    let mut out = String::new();
    for c in decoded.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            c if c.is_ascii() => out.push(c),
            c => out.push_str(&format!("&#x{:X};", c as u32)),
        }
    }
    out
}

/// The members of an array as elements; nested arrays nest (`_oembed_create_xml()`).
fn xml_children(data: &Map<String, Value>) -> String {
    let mut out = String::new();
    for (key, value) in data {
        let key = if key.parse::<u64>().is_ok() { "oembed" } else { key.as_str() };
        match value {
            Value::Object(inner) => out.push_str(&format!("<{key}>{}</{key}>", xml_children(inner))),
            Value::Array(items) => {
                let inner: Map<String, Value> = items.iter().enumerate().map(|(i, v)| (i.to_string(), v.clone())).collect();
                out.push_str(&format!("<{key}>{}</{key}>", xml_children(&inner)));
            }
            Value::String(s) => out.push_str(&format!("<{key}>{}</{key}>", xml_text(s))),
            other => out.push_str(&format!("<{key}>{}</{key}>", xml_text(&other.to_string()))),
        }
    }
    out
}

pub(crate) fn xml_of(data: &Map<String, Value>) -> String {
    format!("<?xml version=\"1.0\"?>\n<oembed>{}</oembed>\n", xml_children(data))
}

impl Rest<'_> {
    /// `GET oembed/1.0/embed`.
    pub(crate) async fn oembed_embed(&self, params: &Params) -> Result<Outcome> {
        let Some(url) = params.get("url") else {
            return Ok(Err(RestError::missing_params(&["url"])));
        };
        let url = url.to_owned();
        let format = if params.get("format") == Some("xml") { "xml" } else { "json" };
        // `absint( $maxwidth )`: a number, else 0.
        let maxwidth = params.get("maxwidth").map_or(600, |w| w.trim().parse::<i64>().map_or(0, |n| n.unsigned_abs()));

        let id = self.site.url_to_postid(&url).await?;
        let not_found = || RestError::new("oembed_invalid_url", "Not Found", 404).after_permission_check();
        if id == 0 {
            return Ok(Err(not_found()));
        }
        let Some(post) = self.site.db.post(id).await? else { return Ok(Err(not_found())) };
        // `is_post_publicly_viewable()` and `is_post_embeddable()`.
        let public_type = match post.post_type.as_str() {
            "post" | "page" => true,
            "attachment" => false,
            other => self.site.types.post_type(other).is_some_and(|t| t.public),
        };
        if !public_type || post.post_status != "publish" {
            return Ok(Err(not_found()));
        }

        let width = maxwidth.clamp(200, 600);
        let height = ((width as f64 / 16.0 * 9.0).ceil() as u64).max(200);
        let mut data = Map::new();
        data.insert("version".into(), Value::from("1.0"));
        data.insert("provider_name".into(), Value::from(self.site.settings.blogname.clone()));
        data.insert("provider_url".into(), Value::from(self.site.home_url("")));
        let author = self.site.db.user(post.post_author).await?;
        data.insert("author_name".into(), Value::from(author.as_ref().map_or_else(|| self.site.settings.blogname.clone(), |a| a.display_name.clone())));
        data.insert("author_url".into(), Value::from(author.as_ref().map_or_else(|| self.site.home_url(""), |a| self.site.author_url(a))));
        data.insert("title".into(), Value::from(the_title_for(self.site, &post)));
        // `get_oembed_response_data_rich()`.
        data.insert("type".into(), Value::from("rich"));
        data.insert("width".into(), Value::from(width));
        data.insert("height".into(), Value::from(height));
        data.insert("html".into(), Value::from(rustpress_theme::embed::post_embed_html(self.site, &self.theme.wp_root, &post, width, height).await?));
        let env = crate::util::env(self);
        let thumbnail = images::post_thumbnail_id(&env, post.id).await?;
        if thumbnail != 0
            && let Some((url, w, h)) = images::attachment_image_src_width(&env, thumbnail, width).await?
        {
            data.insert("thumbnail_url".into(), Value::from(url));
            data.insert("thumbnail_width".into(), Value::from(w));
            data.insert("thumbnail_height".into(), Value::from(h));
        }
        drop(env);
        // The keys come out in the order of the filters: the base data, then the rich members.
        let mut resp = Resp::item(Value::Object(data.clone()));
        if format == "xml" {
            resp.xml = Some(xml_of(&data));
        }
        Ok(Ok(resp))
    }

    /// `GET oembed/1.0/proxy`: only for editors.
    pub(crate) fn oembed_proxy(&self) -> Outcome {
        Err(RestError::new("rest_forbidden", "Sorry, you are not allowed to make proxied oEmbed requests.", 401))
    }
}
