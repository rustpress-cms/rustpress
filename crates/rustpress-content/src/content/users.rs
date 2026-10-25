//! Users: `wp_insert_user()`, `wp_update_user()`, `wp_delete_user()` and roles.

use super::{Content, ContentError, Result};
use crate::sanitize;
use rustpress_core::slug::sanitize_title;
use rustpress_db::Val;
use rustpress_phpser::{Key, Value};

/// The fields `wp_insert_user( $userdata )` takes. `None` keeps the stored value on update.
#[derive(Debug, Clone, Default)]
pub struct UserData {
    pub id: Option<u64>,
    pub user_login: Option<String>,
    /// The new password in the clear.
    pub user_pass: Option<String>,
    pub user_email: Option<String>,
    pub user_url: Option<String>,
    pub display_name: Option<String>,
    pub nickname: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub description: Option<String>,
    pub role: Option<String>,
    pub locale: Option<String>,
    pub admin_color: Option<String>,
    pub show_admin_bar_front: Option<String>,
    pub rich_editing: Option<String>,
    pub syntax_highlighting: Option<String>,
    pub comment_shortcuts: Option<String>,
}

fn level_of(site_roles: &rustpress_auth::Roles, role: &str) -> i64 {
    match role {
        "administrator" => 10,
        "editor" => 7,
        "author" => 2,
        "contributor" => 1,
        "subscriber" => 0,
        other => site_roles.capabilities(other).map_or(0, |caps| (0..=10).rev().find(|n| caps.get(&format!("level_{n}")).copied().unwrap_or(false)).unwrap_or(0) as i64),
    }
}

impl Content<'_> {
    fn user_roles(&self) -> rustpress_auth::Roles {
        let key = format!("{}user_roles", self.writer.table_prefix());
        rustpress_auth::Roles::from_option(&self.site.options.string(&key, ""))
    }

    /// `WP_User::set_role()`: the user has exactly this role (none when empty).
    pub async fn set_user_role(&self, user_id: u64, role: &str) -> Result<()> {
        let prefix = self.writer.table_prefix().to_owned();
        let caps = if role.is_empty() { Value::Array(Vec::new()) } else { Value::Array(vec![(Key::Str(role.as_bytes().to_vec()), Value::Bool(true))]) };
        // The user's own capabilities that are not roles stay.
        let roles = self.user_roles();
        let existing = self.db.user_meta(user_id, &format!("{prefix}capabilities")).await?.and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok());
        let mut items: Vec<(Key, Value)> = Vec::new();
        if let Some(Value::Array(old)) = existing {
            for (key, value) in old {
                if let Key::Str(name) = &key
                    && !roles.contains(&String::from_utf8_lossy(name))
                {
                    items.push((key, value));
                }
            }
        }
        if let Value::Array(new) = caps {
            items.extend(new);
        }
        let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(items))).into_owned();
        self.writer.update_user_meta(user_id, &format!("{prefix}capabilities"), &serialized).await?;
        self.writer.update_user_meta(user_id, &format!("{prefix}user_level"), &level_of(&roles, role).to_string()).await?;
        Ok(())
    }

    /// `wp_insert_user()` and `wp_update_user()`: returns the user ID.
    pub async fn insert_user(&self, data: &UserData) -> Result<u64> {
        let tr = |s: &str| self.site.tr(s);
        let existing = match data.id.filter(|id| *id != 0) {
            Some(id) => Some(self.db.user(id).await?.ok_or_else(|| ContentError::Message(tr("Invalid user ID.")))?),
            None => None,
        };
        let update = existing.is_some();

        let login = match (&existing, &data.user_login) {
            (Some(user), _) => user.user_login.clone(),
            (None, Some(login)) => sanitize::user_login(login, true),
            (None, None) => String::new(),
        };
        if !update {
            if login.is_empty() {
                return Err(ContentError::Message(tr("Cannot create a user with an empty login name.")));
            }
            if login.chars().count() > 60 {
                return Err(ContentError::Message(tr("Username may not be longer than 60 characters.")));
            }
            if self.db.user_by_login(&login).await?.is_some() {
                return Err(ContentError::Message(tr("Sorry, that username already exists!")));
            }
        }
        let email = match &data.user_email {
            Some(email) => crate::sanitize::sanitize_email(email),
            None => existing.as_ref().map(|u| u.user_email.clone()).unwrap_or_default(),
        };
        if email.is_empty() && !update {
            // WordPress allows an empty address only when the caller does not give one.
        }
        if !email.is_empty()
            && let Some(other) = self.db.user_by_email(&email).await?
            && Some(other.id) != existing.as_ref().map(|u| u.id)
        {
            return Err(ContentError::Message(tr("Sorry, that email address is already used!")));
        }

        let nicename = match &existing {
            Some(user) => user.user_nicename.clone(),
            None => {
                let base: String = sanitize_title(&login).chars().take(50).collect();
                let mut nicename = base.clone();
                let mut suffix = 2;
                while self.db.user_nicename_taken(&nicename, 0).await? {
                    let keep: String = base.chars().take(49 - suffix.to_string().len()).collect();
                    nicename = format!("{keep}-{suffix}");
                    suffix += 1;
                }
                nicename
            }
        };
        let display_name = data.display_name.clone().or_else(|| existing.as_ref().map(|u| u.display_name.clone())).unwrap_or_else(|| login.clone());
        let url = match &data.user_url {
            Some(url) if !url.trim().is_empty() => rustpress_core::formatting::esc_url_raw(url.trim()),
            Some(_) => String::new(),
            None => existing.as_ref().map(|u| u.user_url.clone()).unwrap_or_default(),
        };
        let mut fields: Vec<(&str, Val)> = vec![
            ("user_nicename", Val::Str(nicename)),
            ("user_email", Val::Str(email)),
            ("user_url", Val::Str(url)),
            ("display_name", Val::Str(sanitize::text_field(&display_name))),
        ];
        if let Some(password) = data.user_pass.as_deref().filter(|p| !p.is_empty()) {
            fields.push(("user_pass", Val::Str(rustpress_auth::password::hash_password(password))));
        } else if !update {
            return Err(ContentError::Message(tr("Cannot create a user with an empty password.")));
        }
        let user_id = match &existing {
            Some(user) => {
                self.writer.update_rows("users", "ID", user.id, &fields).await?;
                user.id
            }
            None => {
                fields.push(("user_login", Val::Str(login.clone())));
                fields.push(("user_registered", Val::Str(self.gmt_now())));
                fields.push(("user_activation_key", Val::Str(String::new())));
                fields.push(("user_status", Val::Int(0)));
                self.writer.insert_row("users", &fields).await?
            }
        };

        // Meta
        let meta = |key: &str, value: Option<&String>, default: &str| -> Option<(String, String)> {
            match value {
                Some(v) => Some((key.to_owned(), v.clone())),
                None if !update => Some((key.to_owned(), default.to_owned())),
                None => None,
            }
        };
        let nickname = data.nickname.as_ref().map(|n| sanitize::text_field(n));
        let mut pairs: Vec<(String, String)> = Vec::new();
        pairs.extend(meta("nickname", nickname.as_ref().or(if update { None } else { Some(&login) }), &login));
        pairs.extend(meta("first_name", data.first_name.as_ref().map(|v| sanitize::text_field(v)).as_ref(), ""));
        pairs.extend(meta("last_name", data.last_name.as_ref().map(|v| sanitize::text_field(v)).as_ref(), ""));
        pairs.extend(meta("description", data.description.as_ref().map(|v| sanitize::textarea_field(v)).as_ref(), ""));
        pairs.extend(meta("rich_editing", data.rich_editing.as_ref(), "true"));
        pairs.extend(meta("syntax_highlighting", data.syntax_highlighting.as_ref(), "true"));
        pairs.extend(meta("comment_shortcuts", data.comment_shortcuts.as_ref(), "false"));
        pairs.extend(meta("admin_color", data.admin_color.as_ref(), "modern"));
        pairs.extend(meta("show_admin_bar_front", data.show_admin_bar_front.as_ref(), "true"));
        pairs.extend(meta("locale", data.locale.as_ref(), ""));
        if !update {
            pairs.push(("use_ssl".into(), "0".into()));
        }
        for (key, value) in pairs {
            self.writer.update_user_meta(user_id, &key, &value).await?;
        }
        // The role.
        match (&data.role, update) {
            (Some(role), _) => self.set_user_role(user_id, role).await?,
            (None, false) => {
                let default_role = self.site.options.string("default_role", "subscriber");
                self.set_user_role(user_id, &default_role).await?
            }
            (None, true) => {}
        }
        Ok(user_id)
    }

    /// `wp_delete_user()`: the posts go to `reassign`, or are deleted with the user.
    pub async fn delete_user(&self, id: u64, reassign: Option<u64>) -> Result<bool> {
        if self.db.user(id).await?.is_none() {
            return Ok(false);
        }
        let posts = self.db.post_ids_of_author(id).await?;
        match reassign {
            Some(to) if to != 0 && to != id => {
                self.writer.execute(format!("UPDATE {} SET post_author = {to} WHERE post_author = {id}", self.writer.table("posts"))).await?;
                self.writer.execute(format!("UPDATE {} SET link_owner = {to} WHERE link_owner = {id}", self.writer.table("links"))).await.ok();
            }
            _ => {
                for (post_id, _) in posts {
                    self.delete_post(post_id, true).await?;
                }
                self.writer.execute(format!("DELETE FROM {} WHERE link_owner = {id}", self.writer.table("links"))).await.ok();
            }
        }
        self.writer.delete_rows("usermeta", "user_id", id).await?;
        self.writer.delete_rows("users", "ID", id).await?;
        Ok(true)
    }
}
