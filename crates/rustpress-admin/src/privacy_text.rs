//! `WP_Privacy_Policy_Content::get_default_content()`: the suggested privacy policy, as blocks.

use rustpress_core::Site;
use rustpress_core::i18n::sprintf;

/// The policy a new site and `options-privacy.php` start from (no tutorial texts, block markup); `home` is the
/// address of the site it names.
pub fn default_content(site: &Site, home: &str) -> String {
    let tr = |text: &str| site.tr(text);
    let suggested = format!("<strong class=\"privacy-policy-tutorial\">{} </strong>", tr("Suggested text:"));
    let heading = |text: &str| format!("<h2 class=\"wp-block-heading\">{}</h2>", tr(text));
    let first = |text: &str| format!("<p>{suggested}{}</p>", tr(text));
    let more = |text: &str| format!("<p>{}</p>", tr(text));
    let strings = [
        heading("Who we are"),
        format!("<p>{suggested}{}</p>", sprintf(&tr("Our website address is: %s."), &[home])),
        heading("Comments"),
        first("When visitors leave comments on the site we collect the data shown in the comments form, and also the visitor&#8217;s IP address and browser user agent string to help spam detection."),
        more("An anonymized string created from your email address (also called a hash) may be provided to the Gravatar service to see if you are using it. The Gravatar service privacy policy is available here: https://automattic.com/privacy/. After approval of your comment, your profile picture is visible to the public in the context of your comment."),
        heading("Media"),
        first("If you upload images to the website, you should avoid uploading images with embedded location data (EXIF GPS) included. Visitors to the website can download and extract any location data from images on the website."),
        heading("Cookies"),
        first("If you leave a comment on our site you may opt-in to saving your name, email address and website in cookies. These are for your convenience so that you do not have to fill in your details again when you leave another comment. These cookies will last for one year."),
        more("If you visit our login page, we will set a temporary cookie to determine if your browser accepts cookies. This cookie contains no personal data and is discarded when you close your browser."),
        more("When you log in, we will also set up several cookies to save your login information and your screen display choices. Login cookies last for two days, and screen options cookies last for a year. If you select &quot;Remember Me&quot;, your login will persist for two weeks. If you log out of your account, the login cookies will be removed."),
        more("If you edit or publish an article, an additional cookie will be saved in your browser. This cookie includes no personal data and simply indicates the post ID of the article you just edited. It expires after 1 day."),
        heading("Embedded content from other websites"),
        first("Articles on this site may include embedded content (e.g. videos, images, articles, etc.). Embedded content from other websites behaves in the exact same way as if the visitor has visited the other website."),
        more("These websites may collect data about you, use cookies, embed additional third-party tracking, and monitor your interaction with that embedded content, including tracking your interaction with the embedded content if you have an account and are logged in to that website."),
        heading("Who we share your data with"),
        first("If you request a password reset, your IP address will be included in the reset email."),
        heading("How long we retain your data"),
        first("If you leave a comment, the comment and its metadata are retained indefinitely. This is so we can recognize and approve any follow-up comments automatically instead of holding them in a moderation queue."),
        more("For users that register on our website (if any), we also store the personal information they provide in their user profile. All users can see, edit, or delete their personal information at any time (except they cannot change their username). Website administrators can also see and edit that information."),
        heading("What rights you have over your data"),
        first("If you have an account on this site, or have left comments, you can request to receive an exported file of the personal data we hold about you, including any data you have provided to us. You can also request that we erase any personal data we hold about you. This does not include any data we are obliged to keep for administrative, legal, or security purposes."),
        heading("Where your data is sent"),
        first("Visitor comments may be checked through an automated spam detection service."),
    ];
    strings
        .iter()
        .map(|string| {
            if string.starts_with("<p>") {
                format!("<!-- wp:paragraph -->\n{string}\n<!-- /wp:paragraph -->\n")
            } else {
                format!("<!-- wp:heading -->\n{string}\n<!-- /wp:heading -->\n")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn english_policy_is_the_one_of_wordpress() {
        let options = rustpress_core::options::Options::from_pairs([("siteurl", "http://example.com")]);
        let site = rustpress_core::Site::from_parts(rustpress_db::Db::connect_lazy("mysql://nobody@127.0.0.1:1/none", "wp_").expect("lazy pool"), options);
        let expected = include_str!("data/privacy_default_content.html").replace("{{home}}", "http://example.com");
        assert_eq!(super::default_content(&site, "http://example.com"), expected);
    }
}
