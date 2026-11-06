//! `core/playlist` and `core/playlist-track`.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use serde_json::{Map, Value, json};

use crate::Result;
use crate::html::{TagEditor, tags};
use crate::render::BlockCtx;

/// `empty()` of a string attribute: missing, `""` or `"0"`.
fn text<'a>(attrs: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    attrs.get(key).and_then(Value::as_str).filter(|v| !v.is_empty() && *v != "0")
}

/// `$attributes[ $key ] ?? ''`.
fn or_empty<'a>(attrs: &'a Map<String, Value>, key: &str) -> &'a str {
    attrs.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn has_id(attrs: &Map<String, Value>) -> bool {
    attrs.get("id").is_some_and(|id| !matches!(id, Value::Null | Value::Bool(false)) && id.as_u64() != Some(0) && id.as_str() != Some("") && id.as_str() != Some("0"))
}

/// `wp_strip_all_tags()`.
fn strip_all_tags(text: &str) -> String {
    let pattern = regex::Regex::new(r"(?is)<script[^>]*?>.*?</script>|<style[^>]*?>.*?</style>").expect("the pattern is valid");
    rustpress_core::formatting::strip_tags(&pattern.replace_all(text, "")).trim().to_owned()
}

/// `render_block_core_playlist()`.
pub(super) fn playlist(bc: &BlockCtx<'_>) -> Result<String> {
    let site = bc.env.site;
    let playlist_id = bc.env.with_state(|s| s.unique_id("playlist-"));
    let show_artwork = bc.attrs.get("showPlayButtonArtwork").is_some_and(|v| v.as_bool().unwrap_or(false));
    let track_type = bc.env.registry.get("core/playlist-track");

    let mut track_ids: Vec<String> = Vec::new();
    let mut tracks = Map::new();
    for inner in bc.block.inner_blocks.iter().filter(|b| b.name.as_deref() == Some("core/playlist-track")) {
        let attrs = track_type.as_ref().map_or_else(|| inner.attrs.clone(), |bt| bt.prepare_attributes(&inner.attrs));
        if !has_id(&attrs) {
            continue;
        }
        let track_id = format!("track-{}", track_ids.len());
        track_ids.push(track_id.clone());
        let unknown = site.tr("Unknown title");
        let title = text(&attrs, "title").unwrap_or(&unknown);
        let (artist, album) = (or_empty(&attrs, "artist"), or_empty(&attrs, "album"));
        let truthy = |value: &str| !value.is_empty() && value != "0";
        let aria_label = if truthy(artist) && truthy(album) {
            sprintf(&site.trx("%1$s by %2$s from the album %3$s", "track title, artist name, album name"), &[title, artist, album])
        } else {
            title.to_owned()
        };
        tracks.insert(track_id, json!({
            "url": esc_url(or_empty(&attrs, "src")),
            "title": strip_all_tags(title),
            "artist": strip_all_tags(artist),
            "album": strip_all_tags(album),
            "image": esc_url(or_empty(&attrs, "image")),
            "imageAlt": strip_all_tags(or_empty(&attrs, "imageAlt")),
            "ariaLabel": strip_all_tags(&aria_label),
        }));
    }
    if track_ids.is_empty() {
        return Ok(String::new());
    }

    let mut playlists = Map::new();
    playlists.insert(playlist_id.clone(), json!({ "tracks": tracks }));
    bc.env.with_state(|s| {
        s.script_modules.insert("@wordpress/block-library/playlist/view".to_owned());
        s.interactivity.state("core/playlist", json!({ "playlists": playlists }));
    });

    let waveform_attribute = |key: &str, name: &str| text(&bc.attrs, key).map_or_else(String::new, |value| format!(" {name}=\"{}\"", esc_attr(value)));
    let waveform = format!(
        "<div class=\"wp-block-playlist__waveform-player\"{}{}{}{}\n\t\tdata-wp-watch=\"callbacks.initWaveformPlayer\"\n\t\tdata-label-play=\"{}\"\n\t\tdata-label-pause=\"{}\"\n\t\tdata-label-seek=\"{}\"\n\t\tdata-label-seek-value=\"{}\"\n\t></div>",
        waveform_attribute("waveformColor", "data-waveform-player-color"),
        waveform_attribute("waveformGradient", "data-waveform-player-gradient"),
        waveform_attribute("waveformBackgroundColor", "data-waveform-player-background-color"),
        waveform_attribute("waveformBackgroundGradient", "data-waveform-player-background-gradient"),
        esc_attr(&site.tr("Play")),
        esc_attr(&site.tr("Pause")),
        esc_attr(&site.tr("Seek")),
        esc_attr(&site.trx("%1$s of %2$s", "audio current time of total duration")),
    );
    let waveform_style = bc.attrs.get("waveformStyle").cloned().unwrap_or_else(|| Value::from("bars"));

    let mut html = bc.content.clone();
    let mut from = 0;
    let is_figure = |t: &crate::html::Tag| !t.is_closer && t.name == "figure";
    if let Some(figure) = tags(&html, 0).into_iter().find(is_figure) {
        let mut editor = TagEditor::from_tag(&html, figure);
        editor.set_attribute("data-wp-interactive", "core/playlist");
        editor.set_attribute(
            "data-wp-context",
            &crate::php_json::encode(&json!({
                "playlistId": playlist_id,
                "currentId": track_ids[0],
                "isPlaying": false,
                "tracks": track_ids,
                "waveformStyle": waveform_style,
                "showPlayButtonArtwork": show_artwork,
                "labelPauseTrack": site.tr("Pause"),
                "labelSelectTrack": site.tr("Play"),
            })),
        );
        html = editor.finish();
        if let Some(figure) = tags(&html, 0).into_iter().find(is_figure) {
            html.insert_str(figure.end, &waveform);
            from = figure.end + waveform.len();
        }
    }

    // Track IDs are render-time only; the buttons get them after the inner blocks have rendered.
    for track_id in &track_ids {
        let button = tags(&html, from).into_iter().find(|t| !t.is_closer && t.attr("class").is_some_and(|c| c.split_ascii_whitespace().any(|x| x == "wp-block-playlist-track__button")));
        let Some(button) = button else { break };
        let mut editor = TagEditor::from_tag(&html, button);
        editor.set_attribute("data-wp-context", &crate::php_json::encode(&json!({ "trackId": track_id })));
        let (start, _, tag) = editor.tag_replacement();
        from = start + tag.len();
        html = editor.finish();
    }
    Ok(html)
}

/// `render_block_core_playlist_track()`.
pub(super) fn playlist_track(bc: &BlockCtx<'_>) -> Result<String> {
    if !has_id(&bc.attrs) {
        return Ok(String::new());
    }
    let site = bc.env.site;
    let wrapper = bc.wrapper_attributes(&[]);
    let show_images = bc.context.get("showImages").filter(|v| !v.is_null()).is_none_or(|v| v.as_bool().unwrap_or(true));
    let unknown = site.tr("Unknown title");
    let title = text(&bc.attrs, "title").unwrap_or(&unknown);
    let (artist, image, alt, length) = (or_empty(&bc.attrs, "artist"), or_empty(&bc.attrs, "image"), or_empty(&bc.attrs, "imageAlt"), or_empty(&bc.attrs, "length"));
    let truthy = |value: &str| !value.is_empty() && value != "0";

    let mut html = format!("<li {wrapper}><button data-wp-on--click=\"actions.changeTrack\" data-wp-bind--aria-current=\"state.isCurrentTrack\" class=\"wp-block-playlist-track__button\">");
    if show_images && truthy(image) {
        html.push_str(&format!("<img class=\"wp-block-playlist-track__image\" src=\"{}\" alt=\"{}\" />", esc_url(image), esc_attr(alt)));
    }
    html.push_str("<span class=\"wp-block-playlist-track__content\">");
    html.push_str(&format!("<span class=\"wp-block-playlist-track__title\">{}</span>", esc_html(title)));
    if truthy(artist) {
        html.push_str(&format!("<span class=\"wp-block-playlist-track__artist\">{}</span>", esc_html(artist)));
    }
    html.push_str("</span>");
    if truthy(length) {
        html.push_str(&format!("<span class=\"wp-block-playlist-track__length\"><span class=\"screen-reader-text\">{} </span>{}</span>", esc_html(&site.tr("Duration:")), esc_html(length)));
    }
    html.push_str(&format!("<span class=\"screen-reader-text\" data-wp-text=\"state.trackButtonActionLabel\">{}</span></button></li>", esc_html(&site.tr("Play"))));
    Ok(html)
}
