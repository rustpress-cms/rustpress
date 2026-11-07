//! The sizes ported themes register with `add_image_size()`. A theme's PHP does not run, so they are listed here.

/// `(name, width, height, crop)`, in the order of the theme.
pub fn of_theme(stylesheet: &str) -> &'static [(&'static str, u32, u32, bool)] {
    match stylesheet {
        "twentyseventeen" => &[("twentyseventeen-featured-image", 2000, 1200, true), ("twentyseventeen-thumbnail-avatar", 100, 100, true)],
        _ => &[],
    }
}
