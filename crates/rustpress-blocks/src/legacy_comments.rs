//! `comments_template()` pieces shared by classic themes and the legacy comments block: the comment list (`wp_list_comments()` with
//! `Walker_Comment`), the form (`comment_form()`) and the notes around them.

use crate::content::{comment_text, get_avatar, the_title_for};
use rustpress_core::formatting::{esc_attr, esc_url};
use rustpress_core::urls::add_query_arg;
use rustpress_core::Site;
use rustpress_db::{Comment, Post};

use crate::Result;

/// What a theme decides about the comment list (`wp_list_comments()` arguments).
pub struct ListArgs {
    pub avatar_size: u64,
    /// The reply link text (already with the theme's icon).
    pub reply_text: String,
    pub short_ping: bool,
}

/// The counters `get_comment_class()` keeps in globals.
#[derive(Default)]
struct Counters {
    alt: u64,
    thread_alt: u64,
}

/// `comments_open()`.
pub fn comments_open(post: &Post) -> bool {
    post.comment_status == "open"
}

/// The walker output of `wp_list_comments()` (without the surrounding `<ol>`).
pub async fn list_comments(site: &Site, env: &crate::env::Env<'_>, post: &Post, comments: &[Comment], args: &ListArgs) -> Result<String> {
    let max_depth: i64 = if site.options.get("thread_comments") == Some("1") { site.options.u64("thread_comments_depth", 5) as i64 } else { -1 };
    let mut out = String::new();
    let mut counters = Counters::default();
    let top: Vec<&Comment> = comments.iter().filter(|c| c.comment_parent == 0 || !comments.iter().any(|p| p.comment_id == c.comment_parent)).collect();
    for comment in top {
        display(site, env, post, comments, comment, 0, max_depth, args, &mut counters, &mut out).await?;
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn display<'a>(
    site: &'a Site,
    env: &'a crate::env::Env<'a>,
    post: &'a Post,
    all: &'a [Comment],
    comment: &'a Comment,
    depth: i64,
    max_depth: i64,
    args: &'a ListArgs,
    counters: &'a mut Counters,
    out: &'a mut String,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let children: Vec<&Comment> = all.iter().filter(|c| c.comment_parent == comment.comment_id).collect();
        let nested = max_depth <= 0 || max_depth > depth + 1;
        start_element(site, env, post, comment, depth + 1, max_depth, !children.is_empty() && nested, args, counters, out).await?;
        if nested && !children.is_empty() {
            out.push_str("<ol class=\"children\">\n");
            for child in &children {
                display(site, env, post, all, child, depth + 1, max_depth, args, counters, out).await?;
            }
            out.push_str("</ol><!-- .children -->\n");
        }
        out.push_str("</li><!-- #comment-## -->\n");
        if !nested {
            for child in &children {
                display(site, env, post, all, child, depth, max_depth, args, counters, out).await?;
            }
        }
        Ok(())
    })
}

/// `get_comment_class()`.
fn comment_class(site: &Site, post: &Post, comment: &Comment, depth: i64, extra: &str, counters: &mut Counters) -> String {
    let _ = site;
    let mut classes: Vec<String> = vec![if comment.comment_type.is_empty() { "comment".to_owned() } else { comment.comment_type.clone() }];
    if comment.user_id != 0 {
        classes.push("byuser".into());
        classes.push(format!("comment-author-{}", comment.user_id));
        if comment.user_id == post.post_author {
            classes.push("bypostauthor".into());
        }
    }
    if counters.alt % 2 == 1 {
        classes.push("odd".into());
        classes.push("alt".into());
    } else {
        classes.push("even".into());
    }
    counters.alt += 1;
    if depth == 1 {
        if counters.thread_alt % 2 == 1 {
            classes.push("thread-odd".into());
            classes.push("thread-alt".into());
        } else {
            classes.push("thread-even".into());
        }
        counters.thread_alt += 1;
    }
    classes.push(format!("depth-{depth}"));
    if !extra.is_empty() {
        classes.push(extra.to_owned());
    }
    classes.iter().map(|c| esc_attr(c)).collect::<Vec<_>>().join(" ")
}

/// `get_comment_author()`.
fn comment_author(site: &Site, comment: &Comment) -> String {
    if comment.comment_author.is_empty() { site.tr("Anonymous") } else { comment.comment_author.clone() }
}

/// `get_comment_author_link()`.
fn comment_author_link(site: &Site, comment: &Comment) -> String {
    let name = comment_author(site, comment);
    let url = comment.comment_author_url.trim();
    if url.is_empty() || url == "http://" || url == "https://" {
        name
    } else {
        format!("<a href=\"{}\" class=\"url\" rel=\"ugc external nofollow\">{name}</a>", esc_url(url))
    }
}

/// `Walker_Comment::html5_comment()` (up to the `<li>`'s end, which the caller adds).
#[allow(clippy::too_many_arguments)]
async fn start_element(
    site: &Site,
    env: &crate::env::Env<'_>,
    post: &Post,
    comment: &Comment,
    depth: i64,
    max_depth: i64,
    has_children: bool,
    args: &ListArgs,
    counters: &mut Counters,
    out: &mut String,
) -> Result<()> {
    let class = comment_class(site, post, comment, depth, if has_children { "parent" } else { "" }, counters);
    let avatar = if args.avatar_size != 0 { get_avatar(env, &comment.comment_author_email, args.avatar_size, "", "", "") } else { String::new() };
    let says = rustpress_core::i18n::sprintf(&site.tr("%s <span class=\"says\">says:</span>"), &[&format!("<b class=\"fn\">{}</b>", comment_author_link(site, comment))]);
    let ts = site.timezone.local_timestamp(&comment.comment_date).unwrap_or(0);
    let date = site.date(&site.options.string("date_format", "F j, Y"), ts);
    let time = site.date(&site.options.string("time_format", "g:i a"), ts);
    let when = rustpress_core::i18n::sprintf(&site.tr("%1$s at %2$s"), &[&date, &time]);
    let link = format!("{}#comment-{}", site.permalink(post).await?, comment.comment_id);
    let text = comment_text(site, &comment.comment_content);
    let reply = if comments_open(post) && (max_depth <= 0 || depth < max_depth) {
        let author = comment_author(site, comment);
        let reply_to = rustpress_core::i18n::sprintf(&site.tr("Reply to %s"), &[&author]);
        let href = format!("{}#respond", esc_url(&add_query_arg(&site.permalink(post).await?, "replytocom", &comment.comment_id.to_string())));
        format!(
            "<div class=\"reply\"><a rel=\"nofollow\" class=\"comment-reply-link\" href=\"{href}\" data-commentid=\"{id}\" data-postid=\"{post_id}\" data-belowelement=\"div-comment-{id}\" data-respondelement=\"respond\" data-replyto=\"{to}\" aria-label=\"{to}\">{text}</a></div>",
            id = comment.comment_id,
            post_id = post.id,
            to = esc_attr(&reply_to),
            text = args.reply_text
        )
    } else {
        String::new()
    };
    out.push_str(&format!(
        "\t\t<li id=\"comment-{id}\" class=\"{class}\">\n\t\t\t<article id=\"div-comment-{id}\" class=\"comment-body\">\n\t\t\t\t<footer class=\"comment-meta\">\n\t\t\t\t\t<div class=\"comment-author vcard\">\n\t\t\t\t\t\t{avatar}\t\t\t\t\t\t{says}\t\t\t\t\t</div><!-- .comment-author -->\n\n\t\t\t\t\t<div class=\"comment-metadata\">\n\t\t\t\t\t\t<a href=\"{link}\"><time datetime=\"{iso}\">{when}</time></a>\t\t\t\t\t</div><!-- .comment-metadata -->\n\n\t\t\t\t\t\t\t\t\t</footer><!-- .comment-meta -->\n\n\t\t\t\t<div class=\"comment-content\">\n\t\t\t\t\t{text}\t\t\t\t</div><!-- .comment-content -->\n\n\t\t\t\t{reply}\t\t\t</article><!-- .comment-body -->\n\t\t",
        id = comment.comment_id,
        link = esc_url(&link),
        iso = site.date("c", ts),
    ));
    Ok(())
}

/// `comment_form()` with the defaults of a theme that supports `html5` comment forms.
pub async fn comment_form(site: &Site, post: &Post, request_uri: &str, block_theme: bool) -> Result<String> {
    if !comments_open(post) {
        return Ok(String::new());
    }
    let options = &site.options;
    let require = options.get("require_name_email") != Some("0");
    let indicator = format!("<span class=\"required\">{}</span>", rustpress_core::formatting::esc_html(&site.tr("*")));
    let required_attr = if require { " required" } else { "" };
    let mark = if require { format!(" {indicator}") } else { String::new() };
    let notes = format!(
        "<p class=\"comment-notes\"><span id=\"email-notes\">{}</span> <span class=\"required-field-message\">{}</span></p>",
        site.tr("Your email address will not be published."),
        rustpress_core::i18n::sprintf(&site.tr("Required fields are marked %s"), &[&indicator])
    );
    let comment_field = format!(
        "<p class=\"comment-form-comment\"><label for=\"comment\">{}{}</label> <textarea id=\"comment\" name=\"comment\" cols=\"45\" rows=\"8\" maxlength=\"65525\" required></textarea></p>",
        site.trx("Comment", "noun"),
        format_args!(" {indicator}")
    );
    let mut fields = vec![
        format!("<p class=\"comment-form-author\"><label for=\"author\">{}{mark}</label> <input id=\"author\" name=\"author\" type=\"text\" value=\"\" size=\"30\" maxlength=\"245\" autocomplete=\"name\"{required_attr} /></p>", site.tr("Name")),
        format!("<p class=\"comment-form-email\"><label for=\"email\">{}{mark}</label> <input id=\"email\" name=\"email\" type=\"email\" value=\"\" size=\"30\" maxlength=\"100\" aria-describedby=\"email-notes\" autocomplete=\"email\"{required_attr} /></p>", site.tr("Email")),
        format!("<p class=\"comment-form-url\"><label for=\"url\">{}</label> <input id=\"url\" name=\"url\" type=\"url\" value=\"\" size=\"30\" maxlength=\"200\" autocomplete=\"url\" /></p>", site.tr("Website")),
    ];
    if options.get("show_comments_cookies_opt_in") == Some("1") {
        fields.push(format!(
            "<p class=\"comment-form-cookies-consent\"><input id=\"wp-comment-cookies-consent\" name=\"wp-comment-cookies-consent\" type=\"checkbox\" value=\"yes\" /> <label for=\"wp-comment-cookies-consent\">{}</label></p>",
            site.tr("Save my name, email, and website in this browser for the next time I comment.")
        ));
    }
    let cancel = if options.get("thread_comments") == Some("1") {
        let href = format!("{}#respond", esc_url(&rustpress_core::urls::remove_query_arg(request_uri, "replytocom")));
        format!(" <small><a rel=\"nofollow\" id=\"cancel-comment-reply-link\" href=\"{href}\" style=\"display:none;\">{}</a></small>", site.tr("Cancel reply"))
    } else {
        String::new()
    };
    let (field_class, button_class) = if block_theme { (" wp-block-button", "submit wp-block-button__link wp-element-button") } else { ("", "submit") };
    let submit = format!(
        "<p class=\"form-submit{field_class}\"><input name=\"submit\" type=\"submit\" id=\"submit\" class=\"{button_class}\" value=\"{}\" /> <input type='hidden' name='comment_post_ID' value='{}' id='comment_post_ID' />\n<input type='hidden' name='comment_parent' id='comment_parent' value='0' />\n</p>",
        esc_attr(&site.tr("Post Comment")),
        post.id
    );
    Ok(format!(
        "\t<div id=\"respond\" class=\"comment-respond\">\n\t\t<h3 id=\"reply-title\" class=\"comment-reply-title\">{}{cancel}</h3><form action=\"{}\" method=\"post\" id=\"commentform\" class=\"comment-form\">{notes}{comment_field}{}{submit}</form>\t</div><!-- #respond -->\n\t",
        site.tr("Leave a Reply"),
        esc_url(&site.site_url("/wp-comments-post.php")),
        fields.join("\n") + "\n"
    ))
}

/// `comments_template()` with the `theme-compat/comments.php` that WordPress uses for themes
/// without a `comments.php`, as the legacy comments block renders it.
pub async fn compat_comments_template(site: &Site, env: &crate::env::Env<'_>, post: &Post, request_uri: &str, block_theme: bool) -> Result<String> {
    if crate::content::post_password_required(post) {
        return Ok(format!("\t\t<p class=\"nocomments\">{}</p>\n\t", site.tr("This post is password protected. Enter the password to view comments.")));
    }
    let comments = site.db.approved_comments(post.id).await?;
    let mut out = String::from("\n<!-- You can start editing here. -->\n\n");
    if !comments.is_empty() {
        let title = format!("&#8220;{}&#8221;", the_title_for(site, post));
        let count = post.comment_count;
        let heading = if count == 1 {
            rustpress_core::i18n::sprintf(&site.tr("One response to %s"), &[&title])
        } else {
            rustpress_core::i18n::sprintf(&site.trn("%1$s response to %2$s", "%1$s responses to %2$s", count), &[&site.number_i18n(count.max(0) as u64), &title])
        };
        let navigation = "\t<div class=\"navigation\">\n\t\t<div class=\"alignleft\"></div>\n\t\t<div class=\"alignright\"></div>\n\t</div>\n";
        let args = ListArgs { avatar_size: 32, reply_text: site.tr("Reply"), short_ping: false };
        out.push_str(&format!("\t<h3 id=\"comments\">\n\t\t{heading}\t</h3>\n\n{navigation}\n\t<ol class=\"commentlist\">\n\t"));
        out.push_str(&list_comments(site, env, post, &comments, &args).await?);
        out.push_str(&format!("\t</ol>\n\n{navigation}\n"));
    } else if comments_open(post) {
        out.push_str("\n\t\t\t<!-- If comments are open, but there are no comments. -->\n\n\t\n");
    } else {
        out.push_str(&format!("\n\t\n\t\t<!-- If comments are closed. -->\n\t\t<p class=\"nocomments\">{}</p>\n\t", site.tr("Comments are closed.")));
    }
    out.push_str(&comment_form(site, post, request_uri, block_theme).await?);
    Ok(out)
}