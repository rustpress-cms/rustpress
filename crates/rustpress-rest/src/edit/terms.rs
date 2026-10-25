//! `wp/v2/categories`, `tags` and the routes of custom taxonomies for signed-in users
//! (`WP_REST_Terms_Controller`).

use rustpress_content::content::TermData;
use serde_json::json;

use super::Args;
use crate::error::RestError;
use crate::params::Params;
use crate::terms::Tax;
use crate::{Outcome, Resp, Rest, Result};

impl Rest<'_> {
    fn can_manage_terms(&self, _tax: &Tax) -> bool {
        self.can("manage_categories")
    }

    pub(crate) async fn terms_custom(&self, base: &str, id: Option<&str>, method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let Some(tax) = self.site.types.taxonomies.iter().find(|t| t.show_in_rest && t.rest_base() == base).and_then(|t| self.tax(&t.name)) else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        self.terms_route(&tax, id, method, args, params).await
    }

    pub(crate) async fn terms_route(&self, tax: &Tax, id: Option<&str>, method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let view = {
            let mut p = params.clone();
            p.set("context", "view");
            p
        };
        match (method, id) {
            ("GET", None) => {
                let mut outcome = self.terms_collection(tax, &view).await?;
                if let Ok(resp) = &mut outcome {
                    resp.allow = Some(if self.can_manage_terms(tax) { "GET, POST" } else { "GET" }.to_owned());
                }
                Ok(outcome)
            }
            ("GET", Some(id)) => {
                let mut outcome = self.terms_item(tax, id, &view).await?;
                if let Ok(resp) = &mut outcome {
                    resp.allow = Some(if self.can_manage_terms(tax) { "GET, POST, PUT, PATCH, DELETE" } else { "GET" }.to_owned());
                    self.term_target_hints(&mut resp.data, tax);
                }
                Ok(outcome)
            }
            ("POST", None) => {
                if !self.can_manage_terms(tax) {
                    return Ok(Err(self.forbidden("rest_cannot_create", "Sorry, you are not allowed to create terms in this taxonomy.")));
                }
                let Some(name) = args.str("name").filter(|n| !n.trim().is_empty()) else {
                    return Ok(Err(RestError::missing_params(&["name"])));
                };
                let content = match self.content() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(e)),
                };
                let data = TermData { name, slug: args.str("slug").unwrap_or_default(), parent: args.u64("parent").unwrap_or(0), description: args.str("description").unwrap_or_default() };
                let new_id = match content.insert_term(&tax.name, &data).await {
                    Ok(id) => id,
                    Err(e) => {
                        let message = e.to_string();
                        let code = if message.contains("already exists") { "term_exists" } else { "rest_term_error" };
                        let mut error = RestError::new(code, &message, 400);
                        if code == "term_exists" {
                            error = error.after_permission_check();
                        }
                        return Ok(Err(error));
                    }
                };
                self.finish_term(tax, new_id, true).await
            }
            ("POST" | "PUT" | "PATCH", Some(id)) => {
                let id: u64 = id.parse().unwrap_or(0);
                if !self.can_manage_terms(tax) {
                    return Ok(Err(self.forbidden("rest_cannot_update", "Sorry, you are not allowed to edit this term.")));
                }
                let Some(term) = self.site.db.term_by_id(Some(&tax.name), id).await? else {
                    return Ok(Err(RestError::new("rest_term_invalid", "Term does not exist.", 404)));
                };
                let content = match self.content() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(e)),
                };
                let data = TermData {
                    name: args.str("name").unwrap_or_else(|| term.name.clone()),
                    slug: args.str("slug").unwrap_or_else(|| term.slug.clone()),
                    parent: args.u64("parent").unwrap_or(term.parent),
                    description: args.str("description").unwrap_or_else(|| term.description.clone()),
                };
                if let Err(e) = content.update_term(id, &tax.name, &data).await {
                    return Ok(Err(RestError::new("rest_term_error", &e.to_string(), 400)));
                }
                self.finish_term(tax, id, false).await
            }
            ("DELETE", Some(id)) => {
                let id: u64 = id.parse().unwrap_or(0);
                if !self.can_manage_terms(tax) {
                    return Ok(Err(self.forbidden("rest_cannot_delete", "Sorry, you are not allowed to delete this term.")));
                }
                if !args.bool("force").unwrap_or(false) {
                    return Ok(Err(RestError::new("rest_trash_not_supported", "Terms do not support trashing. Set force=true to delete.", 501)));
                }
                let view = {
                    let mut p = Params::parse("context=view");
                    p.set("context", "edit");
                    p
                };
                let _ = view;
                let Some(term) = self.site.db.term_by_id(Some(&tax.name), id).await? else {
                    return Ok(Err(RestError::new("rest_term_invalid", "Term does not exist.", 404)));
                };
                let previous = match self.terms_item(tax, &id.to_string(), &Params::parse("context=view")).await? {
                    Ok(resp) => resp.data,
                    Err(_) => json!({"id": term.term_id}),
                };
                let content = match self.content() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(e)),
                };
                if !matches!(content.delete_term(id, &tax.name).await, Ok(true)) {
                    return Ok(Err(RestError::new("rest_cannot_delete", "The term cannot be deleted.", 500)));
                }
                Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))))
            }
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    fn term_target_hints(&self, data: &mut serde_json::Value, tax: &Tax) {
        if !self.can_manage_terms(tax) {
            return;
        }
        if let Some(serde_json::Value::Array(items)) = data.get_mut("_links").and_then(|l| l.get_mut("self"))
            && let Some(serde_json::Value::Object(link)) = items.first_mut()
        {
            link.insert("targetHints".into(), json!({"allow": ["GET", "POST", "PUT", "PATCH", "DELETE"]}));
        }
    }

    async fn finish_term(&self, tax: &Tax, id: u64, created: bool) -> Result<Outcome> {
        let mut outcome = self.terms_item(tax, &id.to_string(), &Params::parse("context=view")).await?;
        if let Ok(resp) = &mut outcome {
            self.term_target_hints(&mut resp.data, tax);
            if created {
                resp.status = Some(201);
                resp.location = Some(self.rest_url(&format!("wp/v2/{}/{id}", tax.base)));
            }
            resp.allow = Some("GET, POST, PUT, PATCH, DELETE".to_owned());
        }
        Ok(outcome)
    }
}
