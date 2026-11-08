//! The regions of `cmplz_config::load_regions()` and the country lists of the configuration.

/// The keys of `COMPLIANZ::$config->countries`.
pub const COUNTRIES: &[&str] = &[
    "AF", "AX", "AL", "DZ", "AS", "AD", "AO", "AI", "AQ", "AG", "AR", "AM", "AW", "AU", "AT", "AZ", "BS", "BH", "BD",
    "BB", "BY", "BE", "BZ", "BJ", "BM", "BT", "BO", "BA", "BW", "BV", "BR", "IO", "BN", "BG", "BF", "BI", "KH", "CM",
    "CA", "CV", "KY", "CF", "TD", "CL", "CN", "CX", "CC", "CO", "KM", "CG", "CD", "CK", "CR", "CI", "HR", "CU", "CY",
    "CZ", "DK", "DJ", "DM", "DO", "EC", "EG", "SV", "GQ", "ER", "EE", "ET", "FK", "FO", "FJ", "FI", "FR", "GF", "PF",
    "TF", "GA", "GM", "GE", "DE", "GH", "GI", "GR", "GL", "GD", "GP", "GU", "GT", "GG", "GN", "GW", "GY", "HT", "HM",
    "VA", "HN", "HK", "HU", "IS", "IN", "ID", "IR", "IQ", "IE", "IM", "IL", "IT", "JM", "JP", "JE", "JO", "KZ", "KE",
    "KI", "KR", "KW", "KG", "LA", "LV", "LB", "LS", "LR", "LY", "LI", "LT", "LU", "MO", "MK", "MG", "MW", "MY", "MV",
    "ML", "MT", "MH", "MQ", "MR", "MU", "YT", "MX", "FM", "MD", "MC", "MN", "ME", "MS", "MA", "MZ", "MM", "NA", "NR",
    "NP", "NL", "AN", "NC", "NZ", "NI", "NE", "NG", "NU", "NF", "MP", "NO", "OM", "PK", "PW", "PS", "PA", "PG", "PY",
    "PE", "PH", "PN", "PL", "PT", "PR", "QA", "RE", "RO", "RU", "RW", "BL", "SH", "KN", "LC", "MF", "PM", "VC", "WS",
    "SM", "ST", "SA", "SN", "RS", "SC", "SL", "SG", "SK", "SI", "SB", "SO", "ZA", "GS", "ES", "LK", "SD", "SR", "SJ",
    "SZ", "SE", "CH", "SY", "TW", "TJ", "TZ", "TH", "TL", "TG", "TK", "TO", "TT", "TN", "TR", "TM", "TC", "TV", "UG",
    "UA", "AE", "GB", "US", "UM", "UY", "UZ", "VU", "VE", "VN", "VG", "VI", "WF", "EH", "YE", "ZM", "ZW",
];

/// `cmplz_config::load_eu_countries()`.
pub const EU_COUNTRIES: &[&str] = &[
    "BE", "BG", "CY", "DK", "DE", "EE", "FI", "FR", "GR", "HU", "IE", "IT", "IS", "HR", "LV", "LT", "LI", "LU", "MT",
    "NL", "NO", "AT", "PL", "PT", "RO", "SK", "SI", "ES", "CZ", "VL", "SE", "CH", "RE", "GP", "MQ", "GY", "YT", "MF",
];

/// A region of `COMPLIANZ::$config->regions`.
pub struct Region {
    pub code: &'static str,
    /// `type`: the consent type.
    pub kind: &'static str,
    pub statistics_consent: &'static str,
    countries: &'static [&'static str],
}

/// `COMPLIANZ::$config->regions`, in their order.
pub const REGIONS: &[Region] = &[
    Region { code: "us", kind: "optout", statistics_consent: "no", countries: &["US"] },
    Region { code: "ca", kind: "optout", statistics_consent: "no", countries: &["CA"] },
    Region { code: "eu", kind: "optin", statistics_consent: "when_not_anonymous", countries: EU_COUNTRIES },
    Region { code: "uk", kind: "optin", statistics_consent: "always", countries: &["GB"] },
    Region { code: "au", kind: "optout", statistics_consent: "no", countries: &["AU"] },
    Region { code: "za", kind: "optin", statistics_consent: "no", countries: &["ZA"] },
    Region { code: "br", kind: "optin", statistics_consent: "no", countries: &["BR"] },
];

pub fn region(code: &str) -> Option<&'static Region> {
    REGIONS.iter().find(|r| r.code == code)
}

/// `cmplz_get_region_for_country()`.
pub fn region_for_country(country: &str) -> Option<&'static str> {
    REGIONS.iter().find(|r| r.countries.contains(&country)).map(|r| r.code)
}
