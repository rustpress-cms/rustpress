//! Fluid typography: `wp_get_typography_font_size_value()` and its helpers.

use serde_json::Value;

struct SizeUnit {
    value: f64,
    unit: String,
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// Number as PHP prints it after `round( $n, 3 )`.
fn num(v: f64) -> String {
    let r = round3(v);
    if r == r.trunc() { format!("{}", r as i64) } else { format!("{r}").trim_end_matches('0').to_owned() }
}

/// `wp_get_typography_value_and_unit`.
fn value_and_unit(raw: &str, coerce_to: &str) -> Option<SizeUnit> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let raw = if raw.parse::<f64>().is_ok() { format!("{raw}px") } else { raw.to_owned() };
    let split = raw.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let (number, unit) = raw.split_at(split);
    let mut value: f64 = number.parse().ok()?;
    if !matches!(unit, "rem" | "px" | "em") {
        return None;
    }
    let mut unit = unit.to_owned();
    if coerce_to == "px" && (unit == "em" || unit == "rem") {
        value *= 16.0;
        unit = "px".to_owned();
    }
    if unit == "px" && (coerce_to == "em" || coerce_to == "rem") {
        value /= 16.0;
        unit = coerce_to.to_owned();
    }
    if (coerce_to == "em" || coerce_to == "rem") && (unit == "em" || unit == "rem") {
        unit = coerce_to.to_owned();
    }
    Some(SizeUnit { value: round3(value), unit })
}

/// `wp_get_computed_fluid_typography_value`.
fn computed_fluid(min_viewport: &str, max_viewport: &str, min_size: &str, max_size: &str) -> Option<String> {
    let minimum = value_and_unit(min_size, "")?;
    let unit = minimum.unit.clone();
    let maximum = value_and_unit(max_size, &unit)?;
    let minimum_rem = value_and_unit(min_size, "rem")?;
    let max_vw = value_and_unit(max_viewport, &unit)?;
    let min_vw = value_and_unit(min_viewport, &unit)?;
    let denominator = max_vw.value - min_vw.value;
    if denominator == 0.0 {
        return None;
    }
    let offset = format!("{}{}", num(min_vw.value / 100.0), unit);
    let factor = 100.0 * ((maximum.value - minimum.value) / denominator);
    let scaled = round3(factor);
    let scaled = if scaled == 0.0 { 1.0 } else { scaled };
    Some(format!("clamp({min_size}, {}{} + ((1vw - {offset}) * {}), {max_size})", num(minimum_rem.value), minimum_rem.unit, num(scaled)))
}

/// `wp_get_typography_font_size_value`: the size as is, or a `clamp()` when fluid typography applies.
/// `fluid` is the preset's own `fluid` setting.
pub fn fluid_font_size(size: &str, fluid: Option<&Value>, settings: &Value) -> String {
    if fluid == Some(&Value::Bool(false)) || size.is_empty() {
        return size.to_owned();
    }
    let typography = settings.get("typography");
    let global_fluid = typography.and_then(|t| t.get("fluid"));
    let enabled = global_fluid.is_some_and(|f| !crate::style_engine::is_empty(f));
    let preset_fluid = fluid.filter(|f| !crate::style_engine::is_empty(f));
    if !enabled && preset_fluid.is_none() {
        return size.to_owned();
    }
    let fluid_settings = global_fluid.and_then(Value::as_object);
    let min_viewport = fluid_settings.and_then(|f| f.get("minViewportWidth")).and_then(Value::as_str).unwrap_or("320px").to_owned();
    let wide = settings.get("layout").and_then(|l| l.get("wideSize")).and_then(Value::as_str).filter(|w| value_and_unit(w, "").is_some());
    let mut max_viewport = wide.unwrap_or("1600px").to_owned();
    if let Some(v) = fluid_settings.and_then(|f| f.get("maxViewportWidth")).and_then(Value::as_str) {
        max_viewport = v.to_owned();
    }
    let min_font_limit = fluid_settings.and_then(|f| f.get("minFontSize")).and_then(Value::as_str).filter(|v| value_and_unit(v, "").is_some()).unwrap_or("14px").to_owned();

    let explicit_min = preset_fluid.and_then(|f| f.get("min")).and_then(Value::as_str).map(str::to_owned);
    let explicit_max = preset_fluid.and_then(|f| f.get("max")).and_then(Value::as_str).map(str::to_owned);
    let Some(preferred) = value_and_unit(size, "") else { return size.to_owned() };
    let Some(limit) = value_and_unit(&min_font_limit, &preferred.unit) else { return size.to_owned() };

    if explicit_min.is_none() && explicit_max.is_none() && preferred.value <= limit.value {
        return size.to_owned();
    }
    let max_raw = explicit_max.unwrap_or_else(|| format!("{}{}", num(preferred.value), preferred.unit));
    let min_raw = match explicit_min {
        Some(min) => min,
        None => {
            let in_px = if preferred.unit == "px" { preferred.value } else { preferred.value * 16.0 };
            let factor = (1.0 - 0.075 * in_px.log2()).clamp(0.25, 0.75);
            let calculated = round3(preferred.value * factor);
            if calculated <= limit.value { format!("{}{}", num(limit.value), limit.unit) } else { format!("{}{}", num(calculated), preferred.unit) }
        }
    };
    computed_fluid(&min_viewport, &max_viewport, &min_raw, &max_raw).unwrap_or_else(|| size.to_owned())
}

