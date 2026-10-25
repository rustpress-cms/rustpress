//! `wp/v2/categories`, `wp/v2/tags` and the routes of custom taxonomies (`WP_REST_Terms_Controller`).

use rustpress_db::Term;
use serde_json::{Map, Value};

use crate::error::RestError;
use crate::params::Params;
use crate::util::{LinkParams, Links, link_header};
use crate::{Outcome, Resp, Rest, Result};

impl Tax {
    pub(crate) fn category() -> Self {
        Tax { name: "category".into(), base: "categories".into(), hierarchical: true, object_types: vec!["post".into()] }
    }

    pub(crate) fn tag() -> Self {
        Tax { name: "post_tag".into(), base: "tags".into(), hierarchical: false, object_types: vec!["post".into()] }
    }
}

/// A taxonomy as the REST API sees it.
#[derive(Debug, Clone)]
pub(crate) struct Tax {
    pub name: String,
    /// `rest_base`: the last segment of its routes.
    pub base: String,
    pub hierarchical: bool,
    pub object_types: Vec<String>,
}

impl Rest<'_> {
    /// A taxonomy with REST routes: categories, tags and the custom ones with `show_in_rest`.
    pub(crate) fn tax(&self, name: &str) -> Option<Tax> {
        match name {
            "category" => Some(Tax { name: name.into(), base: "categories".into(), hierarchical: true, object_types: vec!["post".into()] }),
            "post_tag" => Some(Tax { name: name.into(), base: "tags".into(), hierarchical: false, object_types: vec!["post".into()] }),
            other => self.site.types.taxonomy(other).filter(|t| t.show_in_rest).map(|t| Tax {
                name: t.name.clone(),
                base: t.rest_base().to_owned(),
                hierarchical: t.hierarchical,
                object_types: t.object_types.clone(),
            }),
        }
    }

    /// The taxonomies with REST routes that are registered for a post type, in registration order.
    pub(crate) fn taxonomies_of(&self, post_type: &str) -> Vec<Tax> {
        let mut out = Vec::new();
        if post_type == "post" {
            out.extend(self.tax("category"));
            out.extend(self.tax("post_tag"));
        }
        for info in &self.site.types.taxonomies {
            if info.show_in_rest && info.object_types.iter().any(|t| t == post_type)
                && let Some(tax) = self.tax(&info.name)
            {
                out.push(tax);
            }
        }
        out
    }

    /// The REST base of a post type, if it has routes.
    pub(crate) fn post_type_base(&self, post_type: &str) -> Option<String> {
        match post_type {
            "post" => Some("posts".into()),
            "page" => Some("pages".into()),
            "attachment" => Some("media".into()),
            other => self.site.types.post_type(other).filter(|t| t.show_in_rest).map(|t| t.rest_base().to_owned()),
        }
    }

    pub(crate) async fn terms_collection(&self, tax: &Tax, params: &Params) -> Result<Outcome> {
        let taxonomy = tax.name.as_str();
        macro_rules! try_param {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(e)),
                }
            };
        }
        if params.get("context") == Some("edit") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit terms in this taxonomy.", 401)));
        }
        let per_page = try_param!(params.integer("per_page", 10, 1, Some(100)));
        let page = try_param!(params.integer("page", 1, 1, None));
        let offset = try_param!(params.integer("offset", 0, 0, None));
        let order = try_param!(params.one_of("order", &["asc", "desc"], "asc"));
        let orderby = try_param!(params.one_of("orderby", &["id", "include", "name", "slug", "include_slugs", "term_group", "description", "count"], "name"));
        let include = params.id_list("include").unwrap_or_default();
        let slugs = params.list("slug").unwrap_or_default();
        let exclude = params.id_list("exclude").unwrap_or_default();
        let hide_empty = params.truthy("hide_empty");
        let search = params.get("search").filter(|s| !s.is_empty()).map(str::to_lowercase);
        let parent = if tax.hierarchical { params.id_list("parent") } else { None };
        let of_post = match params.id_list("post").and_then(|ids| ids.first().copied()) {
            Some(post) => {
                let ids: Vec<u64> = self.site.db.terms_for_post(post, Some(taxonomy)).await?.iter().map(|t| t.term_id).collect();
                Some(ids)
            }
            None => None,
        };

        let all = self.site.db.terms(taxonomy).await?;
        // `hide_empty` keeps a parent whose children have posts (WordPress pads the counts).
        let non_empty: Vec<u64> = {
            let mut keep: Vec<u64> = all.iter().filter(|t| t.count > 0).map(|t| t.term_id).collect();
            if tax.hierarchical {
                let mut frontier = keep.clone();
                while let Some(id) = frontier.pop() {
                    if let Some(parent) = all.iter().find(|t| t.term_id == id).map(|t| t.parent).filter(|p| *p != 0)
                        && !keep.contains(&parent) {
                            keep.push(parent);
                            frontier.push(parent);
                        }
                }
            }
            keep
        };
        let mut terms: Vec<Term> = all
            .into_iter()
            .filter(|t| include.is_empty() || include.contains(&t.term_id))
            .filter(|t| !exclude.contains(&t.term_id))
            .filter(|t| slugs.is_empty() || slugs.contains(&t.slug))
            .filter(|t| !hide_empty || non_empty.contains(&t.term_id))
            .filter(|t| parent.as_ref().is_none_or(|p| p.contains(&t.parent)))
            .filter(|t| of_post.as_ref().is_none_or(|ids| ids.contains(&t.term_id)))
            .filter(|t| search.as_ref().is_none_or(|s| t.name.to_lowercase().contains(s) || t.slug.to_lowercase().contains(s)))
            .collect();
        // Ties keep the lower ID first, also for `order=desc`.
        let descending = order == "desc";
        let direction = |o: std::cmp::Ordering| if descending { o.reverse() } else { o };
        match orderby {
            "id" | "term_group" => terms.sort_by(|a, b| direction(a.term_id.cmp(&b.term_id))),
            "slug" => terms.sort_by(|a, b| direction(a.slug.to_lowercase().cmp(&b.slug.to_lowercase())).then(a.term_id.cmp(&b.term_id))),
            "count" => terms.sort_by(|a, b| direction(a.count.cmp(&b.count)).then(a.term_id.cmp(&b.term_id))),
            "description" => terms.sort_by(|a, b| direction(a.description.to_lowercase().cmp(&b.description.to_lowercase())).then(a.term_id.cmp(&b.term_id))),
            "include" => terms.sort_by_key(|t| include.iter().position(|i| *i == t.term_id)),
            "include_slugs" => terms.sort_by_key(|t| slugs.iter().position(|s| *s == t.slug)),
            _ => terms.sort_by(|a, b| direction(a.name.to_lowercase().cmp(&b.name.to_lowercase())).then(a.term_id.cmp(&b.term_id))),
        }
        // WordPress counts without padding parents, so `X-WP-Total` can be lower than the list.
        let total = if hide_empty { terms.iter().filter(|t| t.count > 0).count() as u64 } else { terms.len() as u64 };
        let max_pages = total.div_ceil(per_page);
        let start = if offset > 0 { offset } else { (page - 1) * per_page } as usize;
        let context = params.get("context").unwrap_or("view");
        let mut items = Vec::new();
        for term in terms.iter().skip(start).take(per_page as usize) {
            items.push(self.prepare_term(term, tax, context).await?);
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(max_pages);
        resp.link = link_header(self, &format!("wp/v2/{}", tax.base), params, &LinkParams { arrays: &["exclude", "include", "slug"], booleans: &["hide_empty"] }, page, max_pages);
        Ok(Ok(resp))
    }

    pub(crate) async fn terms_item(&self, tax: &Tax, id: &str, params: &Params) -> Result<Outcome> {
        let taxonomy = tax.name.as_str();
        let Ok(id) = id.parse::<u64>() else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        if params.get("context") == Some("edit") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit this term.", 401)));
        }
        let Some(term) = self.site.db.term_by_id(Some(taxonomy), id).await? else {
            return Ok(Err(RestError::new("rest_term_invalid", "Term does not exist.", 404)));
        };
        let context = params.get("context").unwrap_or("view");
        Ok(Ok(Resp::item(self.prepare_term(&term, tax, context).await?)))
    }

    async fn prepare_term(&self, term: &Term, tax: &Tax, context: &str) -> Result<Value> {
        let taxonomy = tax.name.as_str();
        let base = tax.base.as_str();
        let mut data = Map::new();
        data.insert("id".into(), Value::from(term.term_id));
        data.insert("count".into(), Value::from(term.count));
        data.insert("description".into(), Value::from(term.description.clone()));
        data.insert("link".into(), Value::from(self.site.term_link(term).await?));
        data.insert("name".into(), Value::from(term.name.clone()));
        data.insert("slug".into(), Value::from(term.slug.clone()));
        data.insert("taxonomy".into(), Value::from(taxonomy));
        if tax.hierarchical {
            data.insert("parent".into(), Value::from(term.parent));
        }
        data.insert("meta".into(), Value::Array(Vec::new()));

        let mut links = Links::new();
        links.self_link(self.rest_url(&format!("wp/v2/{base}/{}", term.term_id)));
        links.add("collection", vec![], self.rest_url(&format!("wp/v2/{base}")));
        links.add("about", vec![], self.rest_url(&format!("wp/v2/taxonomies/{taxonomy}")));
        if tax.hierarchical && term.parent != 0 {
            links.embeddable("up", self.rest_url(&format!("wp/v2/{base}/{}", term.parent)));
        }
        for object_type in &tax.object_types {
            if let Some(type_base) = self.post_type_base(object_type) {
                links.add("wp:post_type", vec![], rustpress_core::urls::add_query_arg(&self.rest_url(&format!("wp/v2/{type_base}")), base, &term.term_id.to_string()));
            }
        }
        data.insert("_links".into(), links.finish());

        if context == "embed" {
            data.retain(|k, _| matches!(k.as_str(), "id" | "link" | "name" | "slug" | "taxonomy" | "_links"));
        }
        Ok(Value::Object(data))
    }
}
