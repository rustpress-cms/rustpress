//! The list of scheduled events in the option `cron`, ported from `wp-includes/cron.php`: `_get_cron_array()`,
//! `_set_cron_array()`, `wp_schedule_single_event()`, `wp_unschedule_event()` and `wp_clear_scheduled_hook()`.
//! Events of hooks RustPress does not run (plugins, update checks) are kept as they are stored.

use rustpress_db::{Db, Writer};
use rustpress_phpser::{Key, Value};

/// `MINUTE_IN_SECONDS * 10`: how close an identical single event counts as a duplicate.
const DUPLICATE_WINDOW: i64 = 600;

/// One hook's events at a timestamp: `md5( serialize( $args ) )` and the stored event (`schedule`, `args`,
/// `interval`).
type Events = Vec<(Vec<u8>, Value)>;

/// `_get_cron_array()`: the timestamps in their stored order, each with its hooks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CronArray {
    pub entries: Vec<(i64, Vec<(Vec<u8>, Events)>)>,
}

impl CronArray {
    /// Reads the option from the database (not from the snapshot, which is older than the writes of the request).
    pub async fn load(db: &Db) -> rustpress_db::Result<Self> {
        Ok(db.option("cron").await?.map(|raw| Self::parse(&raw)).unwrap_or_default())
    }

    /// The option's value, without `version` (an array without it is the format of WordPress before 2.1, which
    /// `_upgrade_cron_array()` would convert; such an array is read as empty).
    pub fn parse(raw: &str) -> Self {
        let Ok(Value::Array(items)) = rustpress_phpser::unserialize(raw.as_bytes()) else { return Self::default() };
        if !items.iter().any(|(k, _)| matches!(k, Key::Str(s) if s == b"version")) {
            return Self::default();
        }
        let mut entries = Vec::new();
        for (key, hooks) in items {
            let timestamp = match key {
                Key::Int(t) => t,
                Key::Str(s) if s == b"version" => continue,
                Key::Str(s) => String::from_utf8_lossy(&s).parse().unwrap_or(0),
            };
            let Value::Array(hooks) = hooks else { continue };
            let hooks = hooks
                .into_iter()
                .map(|(hook, events)| {
                    let events = match events {
                        Value::Array(events) => events.into_iter().map(|(k, v)| (key_bytes(k), v)).collect(),
                        _ => Vec::new(),
                    };
                    (key_bytes(hook), events)
                })
                .collect();
            entries.push((timestamp, hooks));
        }
        Self { entries }
    }

    /// `_set_cron_array()`: the value with `version` 2 as `update_option( 'cron', $cron, true )` stores it.
    pub fn serialize(&self) -> String {
        let mut items: Vec<(Key, Value)> = self
            .entries
            .iter()
            .map(|(timestamp, hooks)| {
                let hooks = hooks.iter().map(|(hook, events)| (Key::Str(hook.clone()), Value::Array(events.iter().map(|(k, v)| (Key::Str(k.clone()), v.clone())).collect()))).collect();
                (Key::Int(*timestamp), Value::Array(hooks))
            })
            .collect();
        items.push((Key::Str(b"version".to_vec()), Value::Int(2)));
        String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(items))).into_owned()
    }

    /// `_set_cron_array()`.
    pub async fn save(&self, writer: &Writer) -> rustpress_db::Result<()> {
        writer.update_option_with_autoload("cron", &self.serialize(), "on").await?;
        Ok(())
    }

    /// Whether an event of the hook is stored for the arguments.
    pub fn has(&self, hook: &str, args: &Value) -> bool {
        let key = args_key(args);
        self.entries.iter().any(|(_, hooks)| hooks.iter().any(|(h, events)| h == hook.as_bytes() && events.iter().any(|(k, _)| *k == key)))
    }

    /// The timestamps up to `now` with the arguments of their events of the hook (`wp_get_ready_cron_jobs()`).
    pub fn due(&self, hook: &str, now: i64) -> Vec<(i64, Value)> {
        let mut due = Vec::new();
        for (timestamp, hooks) in &self.entries {
            if *timestamp > now {
                break;
            }
            for (h, events) in hooks {
                if h == hook.as_bytes() {
                    due.extend(events.iter().map(|(_, event)| (*timestamp, event.get("args").cloned().unwrap_or(Value::Array(Vec::new())))));
                }
            }
        }
        due
    }

    /// `wp_schedule_single_event()` without plugin filters. Returns whether the event was added (`false` for an
    /// identical event within ten minutes).
    pub fn schedule_single_event(&mut self, timestamp: i64, hook: &str, args: Value, now: i64) -> bool {
        if timestamp <= 0 {
            return false;
        }
        let key = args_key(&args);
        let min = if timestamp < now + DUPLICATE_WINDOW { 0 } else { timestamp - DUPLICATE_WINDOW };
        let max = if timestamp < now { now + DUPLICATE_WINDOW } else { timestamp + DUPLICATE_WINDOW };
        for (at, hooks) in &self.entries {
            if *at < min {
                continue;
            }
            if *at > max {
                break;
            }
            if hooks.iter().any(|(h, events)| h == hook.as_bytes() && events.iter().any(|(k, _)| *k == key)) {
                return false;
            }
        }
        let event = Value::Array(vec![(Key::Str(b"schedule".to_vec()), Value::Bool(false)), (Key::Str(b"args".to_vec()), args)]);
        let index = match self.entries.iter().position(|(at, _)| *at == timestamp) {
            Some(index) => index,
            None => {
                self.entries.push((timestamp, Vec::new()));
                self.entries.len() - 1
            }
        };
        let hooks = &mut self.entries[index].1;
        match hooks.iter_mut().find(|(h, _)| h == hook.as_bytes()) {
            Some((_, events)) => match events.iter_mut().find(|(k, _)| *k == key) {
                Some((_, stored)) => *stored = event,
                None => events.push((key, event)),
            },
            None => hooks.push((hook.as_bytes().to_vec(), vec![(key, event)])),
        }
        // `uksort( $crons, 'strnatcasecmp' )`: the timestamps are positive integers, so in numeric order.
        self.entries.sort_by_key(|(at, _)| *at);
        true
    }

    /// `wp_unschedule_event()`: removes the event, then the hook and the timestamp when they are left empty.
    pub fn unschedule_event(&mut self, timestamp: i64, hook: &str, args: &Value) -> bool {
        let key = args_key(args);
        let Some(index) = self.entries.iter().position(|(at, _)| *at == timestamp) else { return false };
        let hooks = &mut self.entries[index].1;
        let Some(hook_index) = hooks.iter().position(|(h, _)| h == hook.as_bytes()) else { return false };
        let events = &mut hooks[hook_index].1;
        let before = events.len();
        events.retain(|(k, _)| *k != key);
        let removed = events.len() != before;
        if events.is_empty() {
            hooks.remove(hook_index);
        }
        if hooks.is_empty() {
            self.entries.remove(index);
        }
        removed
    }

    /// `wp_clear_scheduled_hook()`: every event of the hook with these arguments. Returns how many there were.
    pub fn clear_scheduled_hook(&mut self, hook: &str, args: &Value) -> usize {
        let key = args_key(args);
        let timestamps: Vec<i64> = self.entries.iter().filter(|(_, hooks)| hooks.iter().any(|(h, events)| h == hook.as_bytes() && events.iter().any(|(k, _)| *k == key))).map(|(at, _)| *at).collect();
        for timestamp in &timestamps {
            self.unschedule_event(*timestamp, hook, args);
        }
        timestamps.len()
    }
}

/// `array( $post->ID )`, the arguments of `publish_future_post`.
pub fn post_args(post_id: u64) -> Value {
    Value::Array(vec![(Key::Int(0), Value::Int(post_id as i64))])
}

/// `md5( serialize( $args ) )`.
fn args_key(args: &Value) -> Vec<u8> {
    format!("{:x}", md5::compute(rustpress_phpser::serialize(args))).into_bytes()
}

fn key_bytes(key: Key) -> Vec<u8> {
    match key {
        Key::Int(i) => i.to_string().into_bytes(),
        Key::Str(s) => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALLED: &str = "a:4:{i:1793000000;a:1:{s:16:\"wp_version_check\";a:1:{s:32:\"40cd750bba9870f18aada2478b24840a\";a:3:{s:8:\"schedule\";s:10:\"twicedaily\";s:4:\"args\";a:0:{}s:8:\"interval\";i:43200;}}}i:1793005400;a:1:{s:17:\"wp_update_plugins\";a:1:{s:32:\"40cd750bba9870f18aada2478b24840a\";a:3:{s:8:\"schedule\";s:10:\"twicedaily\";s:4:\"args\";a:0:{}s:8:\"interval\";i:43200;}}}i:1793007200;a:1:{s:16:\"wp_update_themes\";a:1:{s:32:\"40cd750bba9870f18aada2478b24840a\";a:3:{s:8:\"schedule\";s:10:\"twicedaily\";s:4:\"args\";a:0:{}s:8:\"interval\";i:43200;}}}s:7:\"version\";i:2;}";

    #[test]
    fn keeps_what_it_does_not_change() {
        assert_eq!(CronArray::parse(INSTALLED).serialize(), INSTALLED);
    }

    #[test]
    fn schedules_and_clears_like_wordpress() {
        let mut cron = CronArray::parse(INSTALLED);
        // `md5( serialize( array( 5 ) ) )` is the key WordPress stores for post 5.
        assert!(cron.schedule_single_event(1793003000, "publish_future_post", post_args(5), 1792990000));
        // What PHP's `serialize()` gives for the same array after `uksort()`.
        let expected = INSTALLED.replacen("a:4:", "a:5:", 1).replacen(
            "i:1793005400;",
            "i:1793003000;a:1:{s:19:\"publish_future_post\";a:1:{s:32:\"c9059feef497c200e69cb9956a81f005\";a:2:{s:8:\"schedule\";b:0;s:4:\"args\";a:1:{i:0;i:5;}}}}i:1793005400;",
            1,
        );
        assert_eq!(cron.serialize(), expected);
        // The same event again within ten minutes is a duplicate.
        assert!(!cron.schedule_single_event(1793003300, "publish_future_post", post_args(5), 1792990000));
        assert_eq!(cron.due("publish_future_post", 1793003000), vec![(1793003000, post_args(5))]);
        assert_eq!(cron.clear_scheduled_hook("publish_future_post", &post_args(5)), 1);
        assert_eq!(cron.serialize(), INSTALLED);
    }

    #[test]
    fn an_empty_list_keeps_its_version() {
        let mut cron = CronArray::default();
        cron.schedule_single_event(100, "publish_future_post", post_args(1), 50);
        cron.unschedule_event(100, "publish_future_post", &post_args(1));
        assert_eq!(cron.serialize(), "a:1:{s:7:\"version\";i:2;}");
    }
}
