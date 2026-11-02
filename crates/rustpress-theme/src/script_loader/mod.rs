//! The script and style loader of WordPress (`WP_Dependencies`, `WP_Scripts`, `WP_Styles` and the defaults of
//! `wp-includes/script-loader.php`). The administration uses it for its screens; the website prints the core
//! handles plugins depend on with it.

pub mod deps;
pub mod localize;
pub mod registry;
pub mod scripts;
pub mod styles;
pub mod tags;
