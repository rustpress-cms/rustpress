//! Roles and capabilities: `WP_Roles`, `WP_User::get_role_caps()` and `current_user_can()`.

use std::collections::HashMap;

use rustpress_db::{AuthUser, Db};
use rustpress_phpser::Value;

use crate::meta_caps::SiteCaps;

/// A PHP boolean-ish value of a capability map.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty() && s != b"0",
        Value::Null => false,
        _ => true,
    }
}

/// `array( cap => bool )` as a map; later keys win, like in a PHP array.
fn capability_map(value: &Value) -> HashMap<String, bool> {
    let mut out = HashMap::new();
    if let Some(items) = value.as_array() {
        for (key, value) in items {
            let name = match key {
                rustpress_phpser::Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
                rustpress_phpser::Key::Int(i) => i.to_string(),
            };
            out.insert(name, truthy(value));
        }
    }
    out
}

/// The roles of the site: the option `{prefix}user_roles`.
#[derive(Debug, Clone, Default)]
pub struct Roles {
    roles: HashMap<String, HashMap<String, bool>>,
}

impl Roles {
    /// Reads the serialized option; an unreadable value gives no roles.
    pub fn from_option(raw: &str) -> Self {
        let mut roles = HashMap::new();
        if let Ok(value) = rustpress_phpser::unserialize(raw.as_bytes())
            && let Some(items) = value.as_array()
        {
            for (key, role) in items {
                if let rustpress_phpser::Key::Str(name) = key {
                    roles.insert(String::from_utf8_lossy(name).into_owned(), role.get("capabilities").map(capability_map).unwrap_or_default());
                }
            }
        }
        Self { roles }
    }

    pub fn contains(&self, role: &str) -> bool {
        self.roles.contains_key(role)
    }

    /// The capabilities of a role.
    pub fn capabilities(&self, role: &str) -> Option<&HashMap<String, bool>> {
        self.roles.get(role)
    }
}

/// A signed-in user with everything `current_user_can()` needs.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: u64,
    pub login: String,
    pub display_name: String,
    pub email: String,
    /// The roles of the user, in the order of the capabilities meta.
    pub roles: Vec<String>,
    /// `$user->allcaps`: the capabilities of the roles plus the user's own (which include the role names).
    pub allcaps: HashMap<String, bool>,
    /// The scheduler: may do everything and is nobody (ID 0).
    system: bool,
    /// The options `map_meta_cap()` reads.
    pub(crate) site_caps: SiteCaps,
}

impl CurrentUser {
    /// The user background jobs run as (`wp-cron.php` has no current user; every check passes).
    pub fn system() -> Self {
        Self { id: 0, login: String::new(), display_name: String::new(), email: String::new(), roles: Vec::new(), allcaps: HashMap::new(), system: true, site_caps: SiteCaps::default() }
    }

    /// Loads the capabilities of `user`.
    pub async fn load(db: &Db, site_roles: &Roles, user: &AuthUser) -> Result<Self, rustpress_db::Error> {
        let key = format!("{}capabilities", db.table_prefix());
        let stored = match db.user_meta(user.id, &key).await? {
            Some(raw) => rustpress_phpser::unserialize(raw.as_bytes()).unwrap_or(Value::Null),
            None => Value::Null,
        };
        let caps = capability_map(&stored);
        // The role names, in the order of the stored array (a HashMap would lose it).
        let mut roles: Vec<String> = Vec::new();
        for (name, flag) in stored.as_array().unwrap_or_default() {
            if let rustpress_phpser::Key::Str(name) = name {
                let name = String::from_utf8_lossy(name).into_owned();
                if site_roles.contains(&name) && truthy(flag) {
                    roles.push(name);
                }
            }
        }
        let mut allcaps: HashMap<String, bool> = HashMap::new();
        for role in &roles {
            if let Some(role_caps) = site_roles.roles.get(role) {
                for (cap, granted) in role_caps {
                    allcaps.insert(cap.clone(), *granted);
                }
            }
        }
        for (cap, granted) in caps {
            allcaps.insert(cap, granted);
        }
        let options = db.options_named(&["wp_page_for_privacy_policy", "page_on_front", "page_for_posts", "link_manager_enabled"]).await?;
        let number = |name: &str| options.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.trim().parse::<u64>().ok()).unwrap_or(0);
        let site_caps = SiteCaps {
            privacy_page: number("wp_page_for_privacy_policy"),
            page_on_front: number("page_on_front"),
            page_for_posts: number("page_for_posts"),
            link_manager_enabled: number("link_manager_enabled") != 0,
        };
        Ok(Self { id: user.id, login: user.user_login.clone(), display_name: user.display_name.clone(), email: user.user_email.clone(), roles, allcaps, system: false, site_caps })
    }

    /// `current_user_can( $cap )` for a primitive capability.
    pub fn can(&self, capability: &str) -> bool {
        if capability == "do_not_allow" {
            return false;
        }
        if self.system {
            return true;
        }
        // `map_meta_cap()`: capabilities without a post or user in question that stand for another one.
        let capability = match capability {
            "manage_privacy_options" | "view_site_health_checks" | "export_others_personal_data" | "erase_others_personal_data" => "manage_options",
            "setup_network" => "manage_options",
            "customize" => "edit_theme_options",
            // Without the link manager switched on nobody manages links.
            "manage_links" if !self.site_caps.link_manager_enabled => return false,
            other => other,
        };
        self.allcaps.get(capability).copied().unwrap_or(false)
    }

    /// `manage_links` as if the link manager were switched on (`wp_link_manager_disabled_message()`).
    pub fn can_manage_links_if_enabled(&self) -> bool {
        self.system || self.allcaps.get("manage_links").copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles() -> Roles {
        // a:2:{s:13:"administrator";a:2:{s:4:"name";s:13:"Administrator";s:12:"capabilities";a:2:{s:14:"manage_options";b:1;s:4:"read";b:1;}}s:10:"subscriber";a:2:{s:4:"name";s:10:"Subscriber";s:12:"capabilities";a:1:{s:4:"read";b:1;}}}
        Roles::from_option(r#"a:2:{s:13:"administrator";a:2:{s:4:"name";s:13:"Administrator";s:12:"capabilities";a:2:{s:14:"manage_options";b:1;s:4:"read";b:1;}}s:10:"subscriber";a:2:{s:4:"name";s:10:"Subscriber";s:12:"capabilities";a:1:{s:4:"read";b:1;}}}"#)
    }

    #[test]
    fn roles_are_read_from_the_option() {
        let roles = roles();
        assert!(roles.contains("administrator") && roles.contains("subscriber"));
        assert!(!roles.contains("editor"));
        assert_eq!(roles.roles["administrator"].len(), 2);
        assert!(Roles::from_option("garbage").roles.is_empty());
    }

    #[test]
    fn capability_maps_keep_false_values() {
        let value = rustpress_phpser::unserialize(br#"a:2:{s:4:"read";b:1;s:10:"edit_posts";b:0;}"#).unwrap();
        let map = capability_map(&value);
        assert!(map["read"]);
        assert!(!map["edit_posts"]);
    }
}
