//! `wp-admin/users.php`: the list of users, deleting them and changing their roles.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::add_query_arg;
use rustpress_db::{User, UserListQuery};

use super::common::{forbidden, read_only};
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::content::Content;
use crate::list_table::{Column, Table, row_actions, view_link, views};

/// The roles of a user, in stored order, with their translated names.
pub async fn roles_of(ctx: &Ctx<'_>, user_id: u64) -> Vec<(String, String)> {
    let key = format!("{}capabilities", ctx.env.db.table_prefix());
    let names = super::general::role_names(ctx).await;
    let Some(raw) = ctx.env.db.user_meta(user_id, &key).await.ok().flatten() else { return Vec::new() };
    let Ok(rustpress_phpser::Value::Array(items)) = rustpress_phpser::unserialize(raw.as_bytes()) else { return Vec::new() };
    items
        .iter()
        .filter(|(_, v)| v.as_i64() == Some(1))
        .filter_map(|(k, _)| match k {
            rustpress_phpser::Key::Str(s) => Some(String::from_utf8_lossy(s).into_owned()),
            rustpress_phpser::Key::Int(_) => None,
        })
        .filter_map(|slug| names.iter().find(|(s, _)| *s == slug).map(|(s, n)| (s.clone(), ctx.site().trx(n, "User role"))))
        .collect()
}

fn avatar(ctx: &Ctx<'_>, email: &str, size: u64) -> String {
    let url = |s: u64| esc_url(&rustpress_blocks::content::avatar_url(&ctx.site().options, email, s));
    format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-{size} photo' height='{size}' width='{size}' loading='lazy' decoding='async'/>", url(size), url(size * 2))
}

/// `get_edit_user_link( $user_id )`.
pub fn edit_link(ctx: &Ctx<'_>, user_id: u64) -> String {
    if user_id == ctx.user.id { ctx.admin_url("profile.php") } else { ctx.admin_url(&format!("user-edit.php?user_id={user_id}")) }
}

fn origin(ctx: &Ctx<'_>) -> String {
    let siteurl = &ctx.site().settings.siteurl;
    match siteurl.split_once("://") {
        Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or_default()),
        None => siteurl.clone(),
    }
}

fn update_message(ctx: &Ctx<'_>, code: &str) -> Option<(String, bool)> {
    let tr = |s: &str| ctx.tr(s);
    let count = ctx.request.get("count").and_then(|c| c.parse::<i64>().ok()).unwrap_or(1);
    Some(match code {
        "del" | "del_many" => (sprintf(&ctx.site().trn("%s user deleted.", "%s users deleted.", count), &[&ctx.site().number_i18n(count.unsigned_abs())]), false),
        "add" => (tr("New user created."), false),
        "promote" => (tr("Changed roles."), false),
        "err_admin_role" => (tr("The current user&#8217;s role must have user editing capabilities."), true),
        "err_admin_del" => (tr("You cannot delete the current user."), true),
        "remove" => (tr("User removed from this site."), false),
        "newuseremail" => (tr("A confirmation email has been sent to the new email address."), false),
        "resetpassword" => (tr("Password reset links sent."), false),
        "err_resetpassword" => (tr("Password reset links could not be sent."), true),
        _ => return None,
    })
}

/// `users.php`.
pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    if !ctx.can("list_users") {
        return forbidden(ctx, "Sorry, you are not allowed to list users.");
    }
    let action = {
        let a = request.request("action").unwrap_or_default();
        if a.is_empty() || a == "-1" { request.request("action2").unwrap_or_default().to_owned() } else { a.to_owned() }
    };
    if !action.is_empty() {
        if let Some(response) = handle_action(ctx, &action).await {
            return response;
        }
    }
    let db = ctx.env.db;
    let current_url = format!("{}{}", origin(ctx), request.request_uri);
    let role = request.get("role").unwrap_or_default().to_owned();
    let search = request.get("s").map(|s| s.trim().to_owned()).unwrap_or_default();
    let orderby = request.get("orderby").filter(|o| ["login", "email", "registered", "name"].contains(o)).unwrap_or("login").to_owned();
    let desc = request.get("order").is_some_and(|o| o.eq_ignore_ascii_case("desc"));
    let per_page = db.user_meta(ctx.user.id, "users_per_page").await.ok().flatten().and_then(|v| v.parse::<u64>().ok()).filter(|v| *v >= 1).unwrap_or(20);
    let paged = request.get("paged").and_then(|p| p.parse::<u64>().ok()).filter(|p| *p > 0).unwrap_or(1);
    let query = UserListQuery {
        search: (!search.is_empty()).then(|| search.clone()),
        role: (!role.is_empty() && role != "none").then(|| role.clone()),
        no_role: role == "none",
        orderby: orderby.clone(),
        desc,
        limit: per_page,
        offset: (paged - 1) * per_page,
    };
    let (users, total) = db.list_users(&query).await.unwrap_or_default();
    let ids: Vec<u64> = users.iter().map(|u| u.id).collect();
    let type_refs: Vec<&str> = vec!["post"];
    let counts = db.published_post_counts(&ids, &type_refs).await.unwrap_or_default();

    // The role links.
    let all_roles = super::general::role_names(ctx).await;
    let capability_rows = db.user_capability_rows().await.unwrap_or_default();
    let mut role_count: Vec<(String, u64)> = Vec::new();
    let mut none_count = 0u64;
    for (_, caps) in &capability_rows {
        let roles: Vec<String> = match caps.as_deref().and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok()) {
            Some(rustpress_phpser::Value::Array(items)) => items
                .iter()
                .filter(|(_, v)| v.as_i64() == Some(1))
                .filter_map(|(k, _)| match k {
                    rustpress_phpser::Key::Str(s) => Some(String::from_utf8_lossy(s).into_owned()),
                    rustpress_phpser::Key::Int(_) => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        if roles.is_empty() {
            none_count += 1;
        }
        for r in roles {
            match role_count.iter_mut().find(|(n, _)| *n == r) {
                Some((_, n)) => *n += 1,
                None => role_count.push((r, 1)),
            }
        }
    }
    let total_users = capability_rows.len() as u64;
    let mut links: Vec<(String, String)> = Vec::new();
    let label = |text: &str, n: u64| format!("{text} <span class=\"count\">({})</span>", site.number_i18n(n));
    let all_label = sprintf(&site.trnx("All <span class=\"count\">(%s)</span>", "All <span class=\"count\">(%s)</span>", total_users as i64, "users"), &[&site.number_i18n(total_users)]);
    links.push(("all".into(), view_link("users.php", &all_label, role.is_empty())));
    for (slug, name) in all_roles.iter() {
        let n = role_count.iter().find(|(r, _)| r == slug).map_or(0, |(_, n)| *n);
        if n == 0 {
            continue;
        }
        links.push((slug.clone(), view_link(&add_query_arg("users.php", "role", slug), &label(&site.trx(name, "User role"), n), role == *slug)));
    }
    if none_count > 0 {
        links.push(("none".into(), view_link(&add_query_arg("users.php", "role", "none"), &label(&tr("No role"), none_count), role == "none")));
    }

    // The rows.
    let mut rows = String::new();
    for user in &users {
        rows.push_str(&row(ctx, user, counts.iter().find(|(id, _)| *id == user.id).map_or(0, |(_, n)| *n), &role_count).await);
    }
    let columns = vec![
        Column::new("cb", ""),
        Column::new("username", &tr("Username")).sortable("login", false, Some(&tr("Username")), Some("asc")),
        Column::new("name", &tr("Name")),
        Column::new("email", &tr("Email")).sortable("email", false, Some(&tr("E-mail")), None),
        Column::new("role", &tr("Role")),
        Column::new("posts", &tr("Posts")),
    ];
    let mut bulk = Vec::new();
    if ctx.can("delete_users") {
        bulk.push(("delete".to_owned(), tr("Delete")));
    } else if ctx.can("remove_users") {
        bulk.push(("remove".to_owned(), tr("Remove")));
    }
    bulk.push(("resetpassword".to_owned(), tr("Send password reset")));
    let table = Table {
        site,
        plural: "users".into(),
        singular: "user".into(),
        columns,
        primary: "username".into(),
        current_url,
        orderby: request.get("orderby").unwrap_or_default().to_owned(),
        order: if desc { "desc".into() } else { "asc".into() },
        total_items: total,
        per_page,
        current_page: paged,
        bulk_actions: bulk,
        has_items: !users.is_empty(),
    };
    let role_options = |suffix: &str| -> String {
        let options: Vec<String> = all_roles.iter().rev().map(|(slug, name)| format!("\t<option value='{}'>{}</option>", esc_attr(slug), site.trx(name, "User role"))).collect();
        format!(
            "\t<div class=\"alignleft actions\">\n\t\t\t\t<label class=\"screen-reader-text\" for=\"new_role{suffix}\">\n\t\t\t{change}\t\t</label>\n\t\t<select name=\"new_role{suffix}\" id=\"new_role{suffix}\">\n\t\t\t<option value=\"\">{change}</option>\n\t\t\t\n{options}\t\t\t<option value=\"none\">&mdash; {none} &mdash;</option>\n\t\t</select>\n\t\t\t<input type=\"submit\" name=\"changeit{suffix}\" id=\"changeit{suffix}\" class=\"button\" value=\"{btn}\"  />\t\t</div>\n",
            options = options.join("\n"),
            change = tr("Change role to&hellip;"),
            none = tr("No role for this site"),
            btn = tr("Change")
        )
    };
    let extra_top = if ctx.can("promote_users") { role_options("") } else { String::new() };
    let extra_bottom = if ctx.can("promote_users") { role_options("2") } else { String::new() };
    let caption = if desc { tr("Table ordered by Username. Descending.") } else { tr("Table ordered by Username. Ascending.") };
    let nonce_field = ctx.nonce_field("bulk-users");
    let table_html = table.display(&nonce_field, &extra_top, &extra_bottom, &caption, &rows, &tr("Users list"));

    let title = tr("Users");
    let mut h = String::from("\t\t<div class=\"wrap\">\n\t\t<h1 class=\"wp-heading-inline\">\n\t\t\t");
    h.push_str(&format!("{}\t\t</h1>\n\n", esc_html(&title)));
    if ctx.can("create_users") {
        h.push_str(&format!("\t\t<a href=\"{}\" class=\"page-title-action\">{}</a>\n", esc_url(&ctx.admin_url("user-new.php")), tr("Add User")));
    }
    if !search.is_empty() {
        h.push_str(&format!("\t\t<span class=\"subtitle\">{}</span>\n", sprintf(&tr("Search results for: %s"), &[&format!("<strong>{}</strong>", esc_html(&search))])));
    }
    h.push_str("\t\t<hr class=\"wp-header-end\">\n\n");
    if let Some(code) = request.get("update")
        && let Some((message, error)) = update_message(ctx, code)
    {
        h.push_str(&format!("<div id=\"message\" class=\"{} notice is-dismissible\"><p>{message}</p></div>\n", if error { "error" } else { "updated" }));
    }
    h.push_str(&format!("\t\t{}\n", views(&tr("Filter users list"), &links)));
    h.push_str(&format!(
        "\t\t<form method=\"get\">\n\n\t\t<p class=\"search-box\">\n\t<label class=\"screen-reader-text\" for=\"user-search-input\">{}</label>\n\t<input type=\"search\" id=\"user-search-input\" name=\"s\" value=\"{}\" />\n\t\t<input type=\"submit\" id=\"search-submit\" class=\"button button-compact\" value=\"{}\"  /></p>\n\t\t\n{}\t\t{}\n\t\t</form>\n\n\t\t<div class=\"clear\"></div>\n\t\t</div><!-- .wrap -->\n\t\t",
        tr("Search Users:"),
        esc_attr(&search),
        esc_attr(&tr("Search Users")),
        if role.is_empty() { "\t\t\n".to_owned() } else { format!("\t\t\t\t\t<input type=\"hidden\" name=\"role\" value=\"{}\" />\n", esc_attr(&role)) },
        table_html
    ));
    let screen = Screen { file: "users.php".into(), parent_file: "users.php".into(), submenu_file: Some("users.php".into()), title, help: Some(help(ctx)), ..Default::default() };
    render(ctx, &screen, &h, &Assets { scripts: vec!["wp-lists".into()], ..Default::default() }).await
}

fn help(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    Help {
        tabs: vec![HelpTab { id: "overview".into(), title: tr("Overview"), content: format!("<p>{}</p>", tr("This table shows all users associated with this site.")) }],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p>", tr("For more information:"), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    }
}

async fn row(ctx: &Ctx<'_>, user: &User, posts: u64, _role_count: &[(String, u64)]) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    let roles = roles_of(ctx, user.id).await;
    let role_class = roles.first().map(|(s, _)| s.clone()).unwrap_or_default();
    let can_edit = ctx.can("edit_users") || user.id == ctx.user.id;
    let referer = rustpress_core::canonical::rawurlencode(&ctx.request.request_uri);
    let edit_url = if user.id == ctx.user.id { format!("{}?wp_http_referer={referer}", ctx.admin_url("profile.php")) } else { format!("{}?user_id={}&wp_http_referer={referer}", ctx.admin_url("user-edit.php"), user.id) };
    let nonce = ctx.create_nonce("bulk-users");
    let first = db.user_meta(user.id, "first_name").await.ok().flatten().unwrap_or_default();
    let last = db.user_meta(user.id, "last_name").await.ok().flatten().unwrap_or_default();
    let full_name = format!("{first} {last}").trim().to_owned();
    let mut actions: Vec<(String, String)> = Vec::new();
    if can_edit {
        actions.push(("edit".into(), format!("<a href=\"{}\">{}</a>", esc_url(&edit_url), tr("Edit"))));
        if user.id != ctx.user.id && ctx.can("delete_users") {
            actions.push(("delete".into(), format!("<a class='submitdelete' href='users.php?action=delete&amp;user={}&amp;_wpnonce={nonce}'>{}</a>", user.id, tr("Delete"))));
        } else if user.id != ctx.user.id && ctx.can("remove_users") {
            actions.push(("remove".into(), format!("<a class='submitdelete' href='users.php?action=remove&amp;user={}&amp;_wpnonce={nonce}'>{}</a>", user.id, tr("Remove"))));
        }
    }
    if posts > 0 || true {
        let author_url = site.home_url(&format!("author/{}/", user.user_nicename));
        let author_url = if site.settings.permalink_structure.is_empty() { site.home_url(&format!("?author={}", user.id)) } else { author_url };
        actions.push(("view".into(), format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&author_url), esc_attr(&sprintf(&tr("View posts by %s"), &[&user.display_name])), tr("View"))));
    }
    if user.id != ctx.user.id && can_edit {
        actions.push(("resetpassword".into(), format!("<a class='resetpassword' href='users.php?action=resetpassword&amp;users={}&amp;_wpnonce={nonce}'>{}</a>", user.id, tr("Send password reset"))));
    }
    let name_cell = if full_name.is_empty() { format!("<span aria-hidden=\"true\">&#8212;</span><span class=\"screen-reader-text\">{}</span>", tr("Unknown")) } else { esc_html(&full_name) };
    let posts_cell = if posts > 0 {
        format!("<a href=\"edit.php?author={}\" class=\"edit\"><span aria-hidden=\"true\">{posts}</span><span class=\"screen-reader-text\">{}</span></a>", user.id, sprintf(&site.trn("%s post by this author", "%s posts by this author", posts as i64), &[&site.number_i18n(posts)]))
    } else {
        "0".to_owned()
    };
    let role_names: Vec<String> = roles.iter().map(|(_, n)| n.clone()).collect();
    let can_pick = can_edit;
    let checkbox_cell = if can_pick {
        format!("<input type=\"checkbox\" name=\"users[]\" id=\"user_{id}\" class=\"{}\" value=\"{id}\" /><label for=\"user_{id}\"><span class=\"screen-reader-text\">{}</span></label>", esc_attr(&role_class), sprintf(&tr("Select %s"), &[&esc_html(&user.user_login)]), id = user.id)
    } else {
        String::new()
    };
    format!(
        "\n\t<tr id='user-{id}'><td class='check-column'>{checkbox_cell}</td><th class='username column-username has-row-actions column-primary' data-colname=\"{ucol}\" scope=\"row\" aria-label=\"{login_attr}\">{avatar} <strong>{title}</strong><br />{actions}</th><td class='name column-name' data-colname=\"{ncol}\">{name_cell}</td><td class='email column-email' data-colname=\"{ecol}\"><a href='mailto:{email}'>{email}</a></td><td class='role column-role' data-colname=\"{rcol}\">{roles}</td><td class='posts column-posts num' data-colname=\"{pcol}\">{posts_cell}</td></tr>",
        id = user.id,
        ucol = tr("Username"),
        login_attr = esc_attr(&user.user_login),
        avatar = avatar(ctx, &user.user_email, 32),
        title = if can_edit { format!("<a href=\"{}\">{}</a>", esc_url(&edit_url), esc_html(&user.user_login)) } else { esc_html(&user.user_login) },
        actions = row_actions(site, &actions),
        ncol = tr("Name"),
        ecol = tr("Email"),
        email = esc_html(&user.user_email),
        rcol = tr("Role"),
        roles = role_names.join(", "),
        pcol = tr("Posts"),
    )
}

/// The actions of the list: delete, remove, change role, reset password. `None` shows the list.
async fn handle_action(ctx: &Ctx<'_>, action: &str) -> Option<AdminResponse> {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let users: Vec<u64> = {
        let mut ids: Vec<u64> = request.form.iter().chain(request.query.iter()).filter(|(k, _)| k == "users[]").filter_map(|(_, v)| v.parse().ok()).collect();
        if let Some(single) = request.request("user").and_then(|v| v.parse::<u64>().ok()) {
            ids.push(single);
        }
        if let Some(many) = request.request("users").and_then(|v| v.parse::<u64>().ok()) {
            ids.push(many);
        }
        ids
    };
    let site = ctx.site();
    let expired = || super::die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
    let self_redirect = |code: &str, extra: &str| redirect(&ctx.admin_url(&format!("users.php?update={code}{extra}")));
    match action {
        "delete" | "dodelete" | "remove" | "doremove" if !users.is_empty() => {
            if !ctx.can("delete_users") && !ctx.can("remove_users") {
                return Some(die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to delete users."))));
            }
            let confirm = action == "dodelete" || action == "doremove";
            if confirm && !ctx.verify_nonce("delete-users") {
                return Some(expired());
            }
            if !confirm && !ctx.verify_nonce("bulk-users") {
                return Some(expired());
            }
            let ids: Vec<u64> = users.iter().copied().filter(|id| *id != ctx.user.id).collect();
            if ids.is_empty() {
                return Some(self_redirect("err_admin_del", ""));
            }
            if !confirm {
                return Some(delete_confirmation(ctx, &ids).await);
            }
            let Some(writer) = ctx.env.writer else { return Some(read_only(ctx)) };
            let content = Content { site, db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
            let reassign = if request.post_field("delete_option") == Some("reassign") { request.post_field("reassign_user").and_then(|v| v.parse::<u64>().ok()) } else { None };
            let mut deleted = 0;
            for id in &ids {
                if content.delete_user(*id, reassign).await.unwrap_or(false) {
                    deleted += 1;
                }
            }
            return Some(self_redirect(if deleted == 1 { "del" } else { "del_many" }, &format!("&count={deleted}")));
        }
        _ if request.get("changeit").is_some() || request.post_field("changeit").is_some() || request.post_field("changeit2").is_some() => {
            if !ctx.can("promote_users") {
                return Some(die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit this user."))));
            }
            if !ctx.verify_nonce("bulk-users") {
                return Some(expired());
            }
            let new_role = {
                let a = request.post_field("new_role").unwrap_or_default();
                if a.is_empty() { request.post_field("new_role2").unwrap_or_default() } else { a }
            };
            if new_role.is_empty() {
                return Some(redirect(&ctx.admin_url("users.php")));
            }
            let Some(writer) = ctx.env.writer else { return Some(read_only(ctx)) };
            let content = Content { site, db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
            let target = if new_role == "none" { "" } else { new_role };
            let mut errored = false;
            for id in users.iter().filter(|id| **id != ctx.user.id) {
                let _ = content.set_user_role(*id, target).await;
            }
            if users.contains(&ctx.user.id) {
                errored = true;
            }
            return Some(self_redirect(if errored { "err_admin_role" } else { "promote" }, ""));
        }
        "resetpassword" if !users.is_empty() => {
            if !ctx.verify_nonce("bulk-users") {
                return Some(expired());
            }
            // RustPress cannot send e-mail.
            return Some(self_redirect("err_resetpassword", ""));
        }
        _ => {}
    }
    None
}

/// The "Delete Users" confirmation screen.
async fn delete_confirmation(ctx: &Ctx<'_>, ids: &[u64]) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let users = ctx.env.db.users_by_ids(ids).await.unwrap_or_default();
    let mut h = format!("<form method=\"post\" name=\"updateusers\" id=\"updateusers\">\n{}\n<div class=\"wrap\">\n<h1>{}</h1>\n", ctx.nonce_field("delete-users"), tr("Delete Users"));
    h.push_str(&format!("<p>{}</p>\n<ul>\n", if ids.len() > 1 { tr("You have specified these users for deletion:") } else { tr("You have specified this user for deletion:") }));
    for user in &users {
        h.push_str(&format!("<li><input type=\"hidden\" name=\"users[]\" value=\"{id}\" />{}</li>\n", sprintf(&tr("ID #%1$s: %2$s"), &[&user.id.to_string(), &esc_html(&user.user_login)]), id = user.id));
    }
    h.push_str("</ul>\n");
    let others: Vec<User> = ctx.env.db.all_users().await.unwrap_or_default().into_iter().filter(|u| !ids.contains(&u.id)).collect();
    let mut options = String::new();
    for user in &others {
        options.push_str(&format!("\n\t<option value='{}'>{} ({})</option>", user.id, esc_html(&user.display_name), esc_html(&user.user_login)));
    }
    h.push_str(&format!(
        "<fieldset><p><legend>{}</legend></p>\n<ul style=\"list-style:none;\">\n\t<li><input type=\"radio\" id=\"delete_option0\" name=\"delete_option\" value=\"delete\" checked=\"checked\" />\n\t<label for=\"delete_option0\">{}</label></li>\n\t<li><input type=\"radio\" id=\"delete_option1\" name=\"delete_option\" value=\"reassign\" />\n\t<label for=\"delete_option1\">{}</label>\n\t<label for=\"reassign_user\" class=\"screen-reader-text\">{}</label><select name='reassign_user' id='reassign_user'>{options}\n</select></li></ul></fieldset>\n",
        if ids.len() > 1 { tr("What should be done with content owned by these users?") } else { tr("What should be done with content owned by this user?") },
        tr("Delete all content."),
        tr("Attribute all content to:"),
        tr("Select a user")
    ));
    h.push_str("<input type=\"hidden\" name=\"action\" value=\"dodelete\" />\n");
    h.push_str(&format!("<p class=\"submit\"><input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button button-primary\" value=\"{}\"  /></p>\n</div>\n</form>\n", esc_attr(&tr("Confirm Deletion"))));
    let screen = Screen { file: "users.php".into(), parent_file: "users.php".into(), submenu_file: Some("users.php".into()), title: tr("Delete Users"), ..Default::default() };
    render(ctx, &screen, &h, &Assets::default()).await
}
