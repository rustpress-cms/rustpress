//! Template hierarchy: which template names a request may use, most specific
//! first. For block themes these are file names under `templates/` without
//! the `.html` extension. The chain follows WordPress' `get_template_hierarchy()`
//! including the fall-backs (`single` -> `singular` -> `index`).

use crate::query::{Queried, QueryResult};
use crate::rewrite::urldecode;
use crate::settings::Settings;

/// `page_template` is the page's `_wp_page_template` meta value (a custom
/// template slug), if it has one.
pub fn template_candidates(result: &QueryResult, settings: &Settings, page_template: Option<&str>) -> Vec<String> {
    let f = &result.flags;
    let mut out: Vec<String> = Vec::new();
    let mut add = |name: String| {
        if !out.contains(&name) {
            out.push(name);
        }
    };

    if f.is_embed {
        if let Queried::Post(post) = &result.queried {
            add(format!("embed-{}", post.post_type));
        }
        add("embed".into());
    } else if f.is_404 {
        add("404".into());
    } else if f.is_search {
        add("search".into());
    } else {
        if f.is_front_page {
            add("front-page".into());
        }
        if f.is_home {
            add("home".into());
        } else if f.is_post_type_archive {
            if let Queried::PostType(post_type) = &result.queried {
                add(format!("archive-{post_type}"));
            }
            add("archive".into());
        } else if f.is_tax {
            if let Queried::Term(term) = &result.queried {
                add(format!("taxonomy-{}-{}", term.taxonomy, decoded(&term.slug)));
                add(format!("taxonomy-{}-{}", term.taxonomy, term.slug));
                add(format!("taxonomy-{}", term.taxonomy));
            }
            add("taxonomy".into());
            add("archive".into());
        } else if f.is_attachment {
            add("attachment".into());
            add("single".into());
            add("singular".into());
        } else if f.is_single {
            if let Queried::Post(post) = &result.queried {
                add(format!("single-{}-{}", post.post_type, decoded(&post.post_name)));
                add(format!("single-{}-{}", post.post_type, post.post_name));
                add(format!("single-{}", post.post_type));
            }
            add("single".into());
            add("singular".into());
        } else if f.is_page {
            if let Some(template) = page_template.filter(|t| !t.is_empty() && *t != "default") {
                add(template.trim_end_matches(".php").trim_end_matches(".html").to_owned());
            }
            if let Queried::Post(post) = &result.queried {
                add(format!("page-{}", decoded(&post.post_name)));
                add(format!("page-{}", post.post_name));
                add(format!("page-{}", post.id));
            }
            add("page".into());
            add("singular".into());
        } else if f.is_category {
            if let Queried::Term(term) = &result.queried {
                add(format!("category-{}", decoded(&term.slug)));
                add(format!("category-{}", term.slug));
                add(format!("category-{}", term.term_id));
            }
            add("category".into());
            add("archive".into());
        } else if f.is_tag {
            if let Queried::Term(term) = &result.queried {
                add(format!("tag-{}", decoded(&term.slug)));
                add(format!("tag-{}", term.slug));
                add(format!("tag-{}", term.term_id));
            }
            add("tag".into());
            add("archive".into());
        } else if f.is_author {
            if let Queried::User(user) = &result.queried {
                add(format!("author-{}", decoded(&user.user_nicename)));
                add(format!("author-{}", user.user_nicename));
                add(format!("author-{}", user.id));
            }
            add("author".into());
            add("archive".into());
        } else if f.is_date {
            add("date".into());
            add("archive".into());
        } else if f.is_archive {
            add("archive".into());
        }
    }
    add("index".into());
    let _ = settings; // reserved: privacy-policy page, custom front-page handling
    out
}

fn decoded(slug: &str) -> String {
    String::from_utf8_lossy(&urldecode(slug.as_bytes())).into_owned()
}

#[cfg(test)]
mod tests {
    use rustpress_db::{Post, Term, User};

    use super::*;
    use crate::options::Options;
    use crate::query::{Flags, QueryResult};
    use crate::rewrite::QueryVars;

    fn result(flags: Flags, queried: Queried) -> QueryResult {
        let mut r = QueryResult::new(QueryVars::new(), None);
        r.flags = flags;
        r.queried = queried;
        r
    }

    fn settings() -> Settings {
        Settings::from_options(&Options::default())
    }

    fn candidates(flags: Flags, queried: Queried) -> Vec<String> {
        template_candidates(&result(flags, queried), &settings(), None)
    }

    fn post(post_type: &str, name: &str, id: u64) -> Post {
        Post { id, post_type: post_type.into(), post_name: name.into(), ..Post::default() }
    }

    #[test]
    fn blog_front_page() {
        let f = Flags { is_home: true, is_front_page: true, ..Flags::default() };
        assert_eq!(candidates(f, Queried::None), ["front-page", "home", "index"]);
    }

    #[test]
    fn posts_page_is_home_but_not_front() {
        let f = Flags { is_home: true, is_posts_page: true, ..Flags::default() };
        assert_eq!(candidates(f, Queried::None), ["home", "index"]);
    }

    #[test]
    fn single_post_falls_back_through_singular() {
        let f = Flags { is_single: true, ..Flags::default() };
        let c = candidates(f, Queried::Post(Box::new(post("post", "hello-world", 1))));
        assert_eq!(c, ["single-post-hello-world", "single-post", "single", "singular", "index"]);
    }

    #[test]
    fn pages_prefer_custom_template_then_slug_and_id() {
        let f = Flags { is_page: true, ..Flags::default() };
        let r = result(f, Queried::Post(Box::new(post("page", "about", 7))));
        let c = template_candidates(&r, &settings(), Some("page-wide.html"));
        assert_eq!(c, ["page-wide", "page-about", "page-7", "page", "singular", "index"]);
        let c = template_candidates(&r, &settings(), Some("default"));
        assert_eq!(c[0], "page-about");
    }

    #[test]
    fn static_front_page_uses_front_page_then_page() {
        let f = Flags { is_page: true, is_front_page: true, ..Flags::default() };
        let c = candidates(f, Queried::Post(Box::new(post("page", "start", 3))));
        assert_eq!(c, ["front-page", "page-start", "page-3", "page", "singular", "index"]);
    }

    #[test]
    fn archives() {
        let term = Term { term_id: 4, slug: "rust".into(), taxonomy: "category".into(), ..Term::default() };
        let f = Flags { is_category: true, is_archive: true, ..Flags::default() };
        assert_eq!(candidates(f, Queried::Term(term)), ["category-rust", "category-4", "category", "archive", "index"]);

        let user = User { id: 1, user_nicename: "admin".into(), ..User::default() };
        let f = Flags { is_author: true, is_archive: true, ..Flags::default() };
        assert_eq!(candidates(f, Queried::User(user)), ["author-admin", "author-1", "author", "archive", "index"]);

        let f = Flags { is_date: true, is_year: true, is_archive: true, ..Flags::default() };
        assert_eq!(candidates(f, Queried::None), ["date", "archive", "index"]);
    }

    #[test]
    fn not_found_and_search() {
        assert_eq!(candidates(Flags { is_404: true, ..Flags::default() }, Queried::None), ["404", "index"]);
        assert_eq!(candidates(Flags { is_search: true, ..Flags::default() }, Queried::None), ["search", "index"]);
    }

    #[test]
    fn percent_encoded_slugs_get_a_decoded_variant_first() {
        let f = Flags { is_single: true, ..Flags::default() };
        let c = candidates(f, Queried::Post(Box::new(post("post", "caf%c3%a9", 1))));
        assert_eq!(c[0], "single-post-café");
        assert_eq!(c[1], "single-post-caf%c3%a9");
    }
}
