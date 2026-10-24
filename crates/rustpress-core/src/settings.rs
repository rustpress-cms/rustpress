use crate::options::Options;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowOnFront {
    Posts,
    Page,
}

/// The options the request pipeline needs, parsed once at startup.
#[derive(Debug, Clone)]
pub struct Settings {
    pub home: String,
    pub siteurl: String,
    pub blogname: String,
    pub blogdescription: String,
    pub permalink_structure: String,
    pub posts_per_page: u64,
    pub posts_per_rss: u64,
    pub show_on_front: ShowOnFront,
    pub page_on_front: u64,
    pub page_for_posts: u64,
    pub template: String,
    pub stylesheet: String,
    pub sticky_posts: Vec<u64>,
}

impl Settings {
    pub fn from_options(options: &Options) -> Self {
        let sticky_posts = options
            .value("sticky_posts")
            .and_then(|v| v.as_array().map(|items| items.iter().filter_map(|(_, v)| v.as_i64()).collect::<Vec<_>>()))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| u64::try_from(id).ok())
            .collect();
        let template = options.string("template", "");
        Self {
            home: options.string("home", ""),
            siteurl: options.string("siteurl", ""),
            blogname: options.string("blogname", ""),
            blogdescription: options.string("blogdescription", ""),
            permalink_structure: options.string("permalink_structure", ""),
            posts_per_page: options.u64("posts_per_page", 10).max(1),
            posts_per_rss: options.u64("posts_per_rss", 10).max(1),
            show_on_front: if options.get("show_on_front") == Some("page") {
                ShowOnFront::Page
            } else {
                ShowOnFront::Posts
            },
            page_on_front: options.u64("page_on_front", 0),
            page_for_posts: options.u64("page_for_posts", 0),
            stylesheet: options.string("stylesheet", &template),
            template,
            sticky_posts,
        }
    }

    /// Path part of the `home` URL (`/blog` for `https://example.com/blog`).
    pub fn home_path(&self) -> String {
        let rest = self.home.split_once("://").map_or(self.home.as_str(), |(_, rest)| rest);
        match rest.find('/') {
            Some(i) => rest[i..].trim_end_matches('/').to_owned(),
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_path_is_taken_from_the_home_url() {
        let at = |home: &str| Settings::from_options(&Options::from_pairs([("home", home)])).home_path();
        assert_eq!(at("http://localhost:8080"), "");
        assert_eq!(at("https://example.com/"), "");
        assert_eq!(at("https://example.com/blog/"), "/blog");
        assert_eq!(at("https://example.com/a/b"), "/a/b");
    }

    #[test]
    fn defaults_and_sticky_posts() {
        let s = Settings::from_options(&Options::from_pairs([
            ("sticky_posts", "a:2:{i:0;i:5;i:1;i:9;}"),
            ("show_on_front", "page"),
            ("template", "tt5"),
        ]));
        assert_eq!(s.sticky_posts, [5, 9]);
        assert_eq!(s.show_on_front, ShowOnFront::Page);
        assert_eq!(s.posts_per_page, 10);
        assert_eq!(s.stylesheet, "tt5");
    }
}
