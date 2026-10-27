//! Writes of `wp/v2/users/<id>` and `wp/v2/users/me` (`WP_REST_Users_Controller::update_item()`,
//! `delete_item()` and their permission checks).

use rustpress_content::content::UserData;
use serde_json::{Value, json};

use super::Args;
use crate::error::{Msg, RestError};
use crate::{Outcome, Resp, Rest, Result};

macro_rules! bail {
    ($e:expr) => {
        return Ok(Err($e))
    };
}

impl Rest<'_> {
    /// `get_user()`: a positive ID of an existing user.
    async fn rest_user(&self, id: &str) -> Result<std::result::Result<rustpress_db::User, RestError>> {
        let id = id.parse::<i64>().unwrap_or(0);
        let user = if id > 0 { self.site.db.user(id as u64).await? } else { None };
        Ok(user.ok_or_else(|| RestError::new("rest_user_invalid_id", "Invalid user ID.", 404)))
    }

    /// `POST|PUT|PATCH|DELETE /wp/v2/users/<id>`; `id` is the signed-in user for `users/me`.
    pub(crate) async fn users_write(&self, id: &str, method: &str, args: &Args) -> Result<Outcome> {
        // The schema's checks of the arguments come first.
        if method == "DELETE" {
            if args.get("reassign").is_none() {
                bail!(RestError::missing_params(&["reassign"]));
            }
            // `check_reassign()`: a number, or nothing (`false`) to delete the posts with the user.
            if let Some(value) = args.str("reassign")
                && !value.trim().is_empty()
                && value != "false"
                && value != "0"
                && value.trim().parse::<f64>().is_err()
            {
                bail!(RestError::invalid_param_with_data("reassign", "Invalid user parameter(s).", "rest_invalid_param", json!({"status": 400})));
            }
            return self.delete_user_route(id, args).await;
        }
        if let Some(email) = args.str("email")
            && !rustpress_content::sanitize::is_email(&email)
        {
            bail!(RestError::invalid_param("email", "Invalid email address.", "rest_invalid_email"));
        }
        if let Some(password) = args.str("password") {
            if password.is_empty() {
                bail!(RestError::invalid_param_with_data("password", "Passwords cannot be empty.", "rest_user_invalid_password", json!({"status": 400})));
            }
            if password.contains('\\') {
                bail!(RestError::invalid_param_with_data("password", Msg::new("Passwords cannot contain the \"%s\" character.").with(&["\\"]), "rest_user_invalid_password", json!({"status": 400})));
            }
        }
        self.update_user_route(id, args).await
    }

    /// `update_item_permissions_check()` and `update_item()`.
    async fn update_user_route(&self, id: &str, args: &Args) -> Result<Outcome> {
        let user = match self.rest_user(id).await? {
            Ok(u) => u,
            Err(e) => bail!(e),
        };
        let roles = args.list("roles").unwrap_or_default();
        // Changing nothing but the roles asks for `promote_user` alone.
        let mut only_roles = false;
        if !roles.is_empty() {
            if !self.can("promote_users") {
                bail!(self.forbidden("rest_cannot_edit_roles", "Sorry, you are not allowed to edit roles of this user."));
            }
            let mut keys: Vec<&str> = args.map.keys().map(String::as_str).chain(std::iter::once("id")).collect();
            keys.sort_unstable();
            keys.dedup();
            only_roles = keys == ["id", "roles"];
        }
        if !only_roles && !self.user().is_some_and(|u| u.can_edit_user(user.id)) {
            bail!(self.forbidden("rest_cannot_edit", "Sorry, you are not allowed to edit this user."));
        }

        if let Some(email) = args.str("email")
            && let Some(owner) = self.site.db.user_by_email(&email).await?
            && owner.id != user.id
        {
            bail!(RestError::new("rest_user_invalid_email", "Invalid email address.", 400));
        }
        if args.str("username").is_some_and(|u| !u.is_empty() && u != user.user_login) {
            bail!(RestError::new("rest_user_invalid_argument", "Username is not editable.", 400));
        }
        if let Some(slug) = args.str("slug").filter(|s| !s.is_empty() && *s != user.user_nicename)
            && self.site.db.user_by_nicename(&slug).await?.is_some()
        {
            bail!(RestError::new("rest_user_invalid_slug", "Invalid slug.", 400));
        }
        if let Some(error) = self.check_role_update(user.id, &roles).await? {
            bail!(error);
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => bail!(e),
        };
        let data = UserData {
            id: Some(user.id),
            user_pass: args.str("password"),
            user_email: args.str("email"),
            user_url: args.str("url"),
            user_nicename: args.str("slug"),
            display_name: args.str("name"),
            nickname: args.str("nickname"),
            first_name: args.str("first_name"),
            last_name: args.str("last_name"),
            description: args.str("description"),
            locale: args.str("locale"),
            // `$prepared_user->role = false` and `add_role()`: the roles asked for replace the old ones.
            role: roles.first().cloned(),
            ..Default::default()
        };
        if let Err(e) = content.insert_user(&data).await {
            bail!(RestError::new("rest_user_invalid_argument", &e.to_string(), 400));
        }
        if let Some(Value::Object(meta)) = args.get("meta")
            && let Some(preferences) = meta.get("persisted_preferences")
        {
            let writer = match self.writer() {
                Ok(w) => w,
                Err(e) => bail!(e),
            };
            let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&super::users::json_to_php(preferences))).into_owned();
            writer.update_user_meta(user.id, &format!("{}persisted_preferences", self.site.db.table_prefix()), &serialized).await?;
        }
        let updated = self.site.db.user(user.id).await?.unwrap_or(user);
        let mut resp = Resp::item(self.prepare_user_edit(&updated, "edit").await?);
        resp.allow = Some(self.user_allow(updated.id).join(", "));
        Ok(Ok(resp))
    }

    /// `check_role_update()`: known roles the user may give; administrators keep `edit_users` for themselves.
    async fn check_role_update(&self, user_id: u64, roles: &[String]) -> Result<Option<RestError>> {
        if roles.is_empty() {
            return Ok(None);
        }
        let key = format!("{}user_roles", self.site.db.table_prefix());
        let known = rustpress_auth::Roles::from_option(&self.site.options.fetch(&self.site.db, &key).await?.unwrap_or_default());
        for role in roles {
            let Some(caps) = known.capabilities(role) else {
                return Ok(Some(RestError::new("rest_user_invalid_role", &rustpress_core::i18n::sprintf(&self.site.tr("The role %s does not exist."), &[role]), 400)));
            };
            let me = self.user().map_or(0, |u| u.id);
            if me == user_id && !caps.get("edit_users").copied().unwrap_or(false) {
                return Ok(Some(self.forbidden("rest_user_invalid_role", "Sorry, you are not allowed to give users that role.")));
            }
        }
        Ok(None)
    }

    /// `delete_item_permissions_check()` and `delete_item()`: users are deleted for good, their posts go to
    /// `reassign` or with them.
    async fn delete_user_route(&self, id: &str, args: &Args) -> Result<Outcome> {
        let user = match self.rest_user(id).await? {
            Ok(u) => u,
            Err(e) => bail!(e),
        };
        if !self.can("delete_users") {
            bail!(self.forbidden("rest_user_cannot_delete", "Sorry, you are not allowed to delete this user."));
        }
        if !args.bool("force").unwrap_or(false) {
            bail!(RestError::new("rest_trash_not_supported", Msg::new("Users do not support trashing. Set '%s' to delete.").with(&["force=true"]), 501));
        }
        let reassign = args.u64("reassign").filter(|r| *r != 0);
        if let Some(to) = reassign
            && (to == user.id || self.site.db.user(to).await?.is_none())
        {
            bail!(RestError::new("rest_user_invalid_reassign", "Invalid user ID for reassignment.", 400));
        }
        let mut previous = self.prepare_user_edit(&user, "edit").await?;
        if let Some(object) = previous.as_object_mut() {
            object.remove("_links");
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => bail!(e),
        };
        if !content.delete_user(user.id, reassign).await? {
            bail!(RestError::new("rest_cannot_delete", "The user cannot be deleted.", 500));
        }
        Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))))
    }
}
