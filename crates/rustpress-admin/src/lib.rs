//! The administration area of RustPress: the script and style loader of WordPress, the sign-in page
//! and the screens of `wp-admin`.

pub mod deps;
pub mod die;
pub mod login;
pub mod registry;
pub mod scripts;
pub mod styles;
pub mod tags;

pub use scripts::Scripts;
pub use styles::Styles;
