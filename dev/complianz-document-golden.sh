#!/usr/bin/env bash
# Makes the golden file of the legal documents of Complianz: for each scenario (settings, dates and the rows of the
# plugin's cookie and service tables) the real plugin renders its documents with `get_document_html()` (shortcodes
# switched off, so `[cmplz-manage-consent]` stays as text) and `cmplz_used_cookies()`, and
# crates/rustpress-plugins/tests/data/complianz/documents.json keeps the inputs and the outputs.
#
#   dev/complianz-document-golden.sh
#
# Needs the reference (dev/up.sh) with complianz-gdpr 7.5.6 active. It changes the plugin's settings and fills its
# tables; deactivate and remove the plugin afterwards (see PLAN.md, Complianz).
set -euo pipefail
cd "$(dirname "$0")/.."
out=crates/rustpress-plugins/tests/data/complianz/documents.json
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

python3 - "$work" <<'PY'
import json, sys
work = sys.argv[1]
# ID, name, slug, serviceType, thirdParty, privacyStatementURL, language, isTranslationFrom
services = [
    (1, 'Google Analytics', 'google-analytics', 'website statistics', 1, 'https://policies.google.com/privacy', 'en', 0),
    (2, 'Complianz', 'complianz', 'cookie consent management', 0, '', 'en', 0),
    (3, 'YouTube', 'youtube', 'showing videos', 1, '', 'en', 0),
    (4, 'Google Analytics', 'google-analytics', '', 0, '', 'de', 1),
    (5, 'Custom Service', '', '', 0, '', 'en', 0),
    (6, 'Facebook', 'facebook', 'display of recent social posts and/or social share buttons', 1, 'https://www.facebook.com/policy.php', 'en', 0),
]
# ID, name, slug, retention, serviceID, cookieFunction, purpose, language, isTranslationFrom, ignored, deleted,
# isMembersOnly, showOnPolicy
cookies = [
    (1, '_ga', '_ga', '2 years', 1, 'store and count pageviews', 'Statistics', 'en', 0, 0, 0, 0, 1),
    (2, '_gid', '_gid', '1 day', 1, 'store and count pageviews', 'Statistics', 'en', 0, 0, 0, 0, 1),
    (3, 'cmplz_marketing', 'cmplz_marketing', '365 days', 2, 'store cookie consent preferences', 'Functional', 'en', 0, 0, 0, 0, 1),
    (4, 'YSC', 'ysc', 'session', 3, 'store a unique user ID', 'Marketing/Tracking', 'en', 0, 0, 0, 0, 1),
    (5, 'misc_cookie', '', 'session', 0, '', '', 'en', 0, 0, 0, 0, 1),
    (6, 'hidden_cookie', '', '1 year', 1, 'hidden', 'Statistics', 'en', 0, 0, 0, 0, 0),
    (7, 'ignored_cookie', '', '1 year', 1, 'ignored', 'Statistics', 'en', 0, 1, 0, 0, 1),
    (8, 'deleted_cookie', '', '1 year', 1, 'deleted', 'Statistics', 'en', 0, 0, 1, 0, 1),
    (9, 'wp-members', '', 'session', 5, 'members', 'Functional', 'en', 0, 0, 0, 1, 1),
    (10, '_ga', '_ga', '', 4, '', '', 'de', 1, 0, 0, 0, 1),
    (11, 'pref_cookie', '', '30 days', 5, 'remember settings', 'Preferences &amp; settings', 'en', 0, 0, 0, 0, 1),
    (12, '', '', '', 5, '', 'Functional', 'en', 0, 0, 0, 0, 1),
    (13, 'misc_functional', '', 'session', 0, 'store a session', 'Functional', 'en', 0, 0, 0, 0, 1),
    (14, '_fbp', '_fbp', '3 months', 6, 'store last visit', 'Marketing/Tracking', 'en', 0, 0, 0, 0, 1),
]
company = {
    'organisation_name': 'Example Ltd', 'address_company': 'Main Street 1\n1234 AB Town', 'country_company': 'NL',
    'email_company': 'privacy@example.com', 'telephone_company': '+31 20 123 4567',
}
def scenario(name, regions, options, tables=True, dates=True):
    documents = [['cookie-statement', regions], ['privacy-statement', regions], ['cookie-statement-' + regions, ''], ['impressum', 'all']]
    if regions != 'us':
        documents.append(['cookie-statement', 'us'])
    return {
        'name': name,
        'options': {'regions': regions, 'use_cdb_api': 'yes', **options},
        'publish_date': 1793000000 if dates else None,
        'documents_update_date': 1792000000 if dates else None,
        'last_cookie_sync': 1791000000 if dates else None,
        'services': [dict(zip(['ID', 'name', 'slug', 'serviceType', 'thirdParty', 'privacyStatementURL', 'language', 'isTranslationFrom'], s)) for s in services] if tables else [],
        'cookies': [dict(zip(['ID', 'name', 'slug', 'retention', 'serviceID', 'cookieFunction', 'purpose', 'language', 'isTranslationFrom', 'ignored', 'deleted', 'isMembersOnly', 'showOnPolicy'], c)) for c in cookies] if tables else [],
        'documents': documents,
    }
social = {'uses_social_media': 'yes', 'socialmedia_on_site': ['facebook', 'twitter', 'instagram']}
friendly = {'compile_statistics': 'google-analytics', 'compile_statistics_more_info': ['accepted', 'ip-addresses-blocked', 'no-sharing']}
scenarios = [
    scenario('eu', 'eu', {**company, **social, **friendly, 'uses_ad_cookies': 'yes', 'uses_ad_cookies_personalized': 'yes', 'consent_per_service': 'yes', 'use_cdb_links': 'yes'}),
    scenario('eu-minimal', 'eu', {'use_cdb_api': 'no', 'uses_social_media': 'yes', 'compile_statistics': 'no'}, tables=False, dates=False),
    scenario('eu-optin-stats', 'eu', {**company, 'compile_statistics': 'google-analytics', 'uses_ad_cookies': 'yes', 'uses_ad_cookies_personalized': 'no', 'consent_per_service': 'no', 'use_cdb_links': 'no', 'wp_admin_access_users': 'yes'}),
    scenario('us', 'us', {**company, **social, 'us_states': ['cal', 'va', 'co'], 'us_california_optin': 'no', 'purpose_personaldata': {'selling-data-thirdparty': 1}, 'selling-data-thirdparty_data_purpose_us': ['email', 'ip'], 'compile_statistics': 'matomo'}),
    scenario('us-optin', 'us', {**company, 'us_states': [], 'us_california_optin': 'yes', 'compile_statistics': 'no'}),
    scenario('uk', 'uk', {**company, 'uk_consent_regions': 'yes', 'uses_social_media': 'yes', 'socialmedia_on_site': ['facebook'], **friendly, 'consent_per_service': 'yes'}),
    scenario('ca', 'ca', {**company, **social, 'ca_targets_quebec': 'yes', 'compile_statistics': 'google-tag-manager'}),
    scenario('ca-other', 'ca', {**company, 'ca_targets_quebec': 'no', 'compile_statistics': 'no', 'privacy-statement': 'no'}),
    scenario('au', 'au', {**company, **social, 'uses_ad_cookies': 'yes', **friendly}),
    scenario('za', 'za', {**company, 'compile_statistics': 'google-analytics'}),
    scenario('br', 'br', {**company, **social, 'compile_statistics': 'clicky'}),
]
json.dump(scenarios, open(f'{work}/scenarios.json', 'w'))
for i, s in enumerate(scenarios):
    json.dump(s, open(f'{work}/scenario-{i}.json', 'w'))
print(len(scenarios))
PY

count=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))))' "$work/scenarios.json")
for ((i = 0; i < count; i++)); do
  podman cp "$work/scenario-$i.json" rustpress-wp:/tmp/cmplz-scenario.json
  # The settings and the tables, in one process ...
  cat > "$work/setup.php" <<'PHP'
<?php
$s = json_decode( file_get_contents( '/tmp/cmplz-scenario.json' ), true );
update_option( 'cmplz_options', $s['options'] );
foreach ( array( 'publish_date', 'documents_update_date', 'last_cookie_sync' ) as $name ) {
	if ( null === $s[ $name ] ) {
		delete_option( "cmplz_$name" );
	} else {
		update_option( "cmplz_$name", $s[ $name ], false );
	}
}
delete_option( 'cmplz_transients' );
delete_transient( 'cmplz_cookies' );
global $wpdb;
$wpdb->query( "DELETE FROM {$wpdb->prefix}cmplz_cookies" );
$wpdb->query( "DELETE FROM {$wpdb->prefix}cmplz_services" );
foreach ( $s['services'] as $row ) {
	$wpdb->insert( "{$wpdb->prefix}cmplz_services", $row + array( 'category' => '', 'sharesData' => 0, 'secondParty' => 0, 'sync' => 1, 'lastUpdatedDate' => 0 ) );
}
foreach ( $s['cookies'] as $row ) {
	$wpdb->insert( "{$wpdb->prefix}cmplz_cookies", $row + array( 'sync' => 1, 'type' => 'cookie', 'collectedPersonalData' => '', 'isOwnDomainCookie' => 0, 'domain' => 'self', 'lastUpdatedDate' => 0, 'lastAddDate' => 0, 'firstAddDate' => 0 ) );
}
PHP
  dev/php.sh "$work/setup.php" > /dev/null
  # ... and the documents in another, which loads the documents of the new region.
  cat > "$work/render.php" <<'PHP'
<?php
$s = json_decode( file_get_contents( '/tmp/cmplz-scenario.json' ), true );
remove_all_shortcodes();
$documents = array();
foreach ( $s['documents'] as $doc ) {
	$documents[] = array( $doc[0], $doc[1], COMPLIANZ::$document->get_document_html( $doc[0], $doc[1] ?: false ) );
}
$callbacks = array();
foreach ( array( 'cmplz_uses_only_functional_cookies', 'cmplz_cookie_warning_required_stats_eu', 'cmplz_cookie_warning_required_stats_uk', 'cmplz_cookie_warning_required_stats_za', 'cmplz_uses_marketing_cookies', 'cmplz_site_uses_cookie_warning_cats', 'cmplz_eu_site_needs_cookie_warning', 'cmplz_uk_site_needs_cookie_warning', 'cmplz_za_site_needs_cookie_warning', 'cmplz_tcf_active', 'cmplz_cdb_reference_in_policy', 'cmplz_sells_personal_data', 'cmplz_site_shares_data', 'cmplz_uses_statistics' ) as $func ) {
	$callbacks[ $func ] = (bool) $func();
}
$banner = cmplz_get_cookiebanner( cmplz_get_default_banner_id() );
$s['golden'] = array(
	'options'                     => serialize( get_option( 'cmplz_options' ) ),
	'home_url'                    => get_home_url(),
	'site_url'                    => site_url(),
	'plugin_url'                  => CMPLZ_URL,
	'date_format'                 => get_option( 'date_format' ),
	'publish_date'                => cmplz_localize_date( get_option( 'cmplz_publish_date' ) ),
	'checked_date'                => cmplz_localize_date( get_option( 'cmplz_documents_update_date' ) ),
	'sync_date'                   => COMPLIANZ::$banner_loader->get_last_cookie_sync_date(),
	'accept_text'                 => $banner->accept_x ?? '',
	'save_preferences_text'       => $banner->save_preferences_x ?? '',
	'statistics_privacy_friendly' => COMPLIANZ::$banner_loader->statistics_privacy_friendly(),
	'callbacks'                   => $callbacks,
	'used_cookies'                => cmplz_used_cookies(),
	'documents'                   => $documents,
);
echo wp_json_encode( $s, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE );
PHP
  dev/php.sh "$work/render.php" > "$work/out-$i.json"
done
python3 - "$work" "$count" "$out" <<'PY'
import json, sys
work, count, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
scenarios = [json.load(open(f'{work}/out-{i}.json')) for i in range(count)]
with open(out, 'w', newline='') as f:
    json.dump(scenarios, f, indent=1, ensure_ascii=False)
    f.write('\n')
print(f'{count} scenarios, {sum(len(s["golden"]["documents"]) for s in scenarios)} documents')
PY
