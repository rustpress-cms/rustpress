//! `wp/v2/media` (`WP_REST_Attachments_Controller`).

use serde_json::Value;

use crate::params::Params;
use crate::{Outcome, Resp, Rest, Result};

impl Rest<'_> {
    pub(crate) async fn media_collection(&self, _params: &Params) -> Result<Outcome> {
        let mut resp = Resp::item(Value::Array(Vec::new()));
        resp.total = Some(0);
        resp.total_pages = Some(0);
        Ok(Ok(resp))
    }

    pub(crate) async fn media_item(&self, _id: &str, _params: &Params) -> Result<Outcome> {
        Ok(Err(crate::RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)))
    }
}
