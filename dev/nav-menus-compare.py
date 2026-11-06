#!/usr/bin/env python3
"""`wp-admin/nav-menus.php` of Twenty Seventeen on the copy of a real site (dev/up-soli.sh, :8081) and RustPress
(dev/serve-soli.ps1, :3002): the same steps run on each side from the same state, and every page, its headers and the
rows it leaves behind are compared. Both sides get the same IDs, because the counters of the tables are put back between
them. The copy's own menu stays as it was.

    python dev/nav-menus-compare.py [--show N] [--only <part of a step>]"""
import difflib
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('lc', os.path.join(HERE, 'login-compare2.py'))
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

REFERENCE, RUSTPRESS = 8081, 3002
WHOLE = False
HEADERS = ('content-type', 'cache-control', 'expires', 'x-frame-options', 'location', 'referrer-policy', 'content-security-policy', 'x-content-type-options', 'x-robots-tag')


def option(args, name, default):
    if name in args:
        i = args.index(name)
        value = args[i + 1]
        del args[i:i + 2]
        return value
    return default


def php(code):
    """Runs PHP in the copy, with WordPress loaded, and returns what it prints."""
    with tempfile.NamedTemporaryFile('w', suffix='.php', delete=False, encoding='utf-8', newline='') as f:
        f.write(code)
    path = subprocess.run(['wsl', 'wslpath', '-a', f.name.replace('\\', '/')], capture_output=True, text=True).stdout.strip()
    script = f"podman cp '{path}' rustpress-wp-soli:/tmp/rp-snippet.php >/dev/null && podman exec rustpress-wp-soli php -d display_errors=0 -r 'define(\"WP_USE_THEMES\", false); require \"/var/www/html/wp-load.php\"; require \"/tmp/rp-snippet.php\";'"
    output = subprocess.run(['wsl', 'bash', '-c', script], capture_output=True, encoding='utf-8', errors='replace')
    os.unlink(f.name)
    return output.stdout


PREPARE = r'''<?php
if ( ! get_user_by( 'login', 'rpnav' ) ) {
	wp_insert_user( array( 'user_login' => 'rpnav', 'user_pass' => 'rustpress-dev', 'user_email' => 'rpnav@example.com', 'role' => 'administrator' ) );
}
if ( ! get_user_by( 'login', 'rpnaved' ) ) {
	wp_insert_user( array( 'user_login' => 'rpnaved', 'user_pass' => 'rustpress-dev', 'user_email' => 'rpnaved@example.com', 'role' => 'editor' ) );
}
global $wpdb;
$saved = array(
	'mods'    => get_option( 'theme_mods_twentyseventeen' ),
	'options' => get_option( 'nav_menu_options' ),
);
update_option( 'rp_nav_saved', $saved, false );
foreach ( array( 'rpnav-a', 'rpnav-b' ) as $slug ) {
	wp_insert_post( array( 'post_title' => $slug, 'post_name' => $slug, 'post_type' => 'page', 'post_status' => 'publish', 'post_author' => 1 ) );
}
wp_insert_post( array( 'post_title' => 'rpnav-post', 'post_name' => 'rpnav-post', 'post_status' => 'publish', 'post_author' => 1 ) );
$cat = wp_insert_term( 'rpnav-cat', 'category' );
wp_insert_term( 'rpnav-sub', 'category', array( 'parent' => $cat['term_id'] ) );
'''

# Back to the state of `PREPARE`, with the counters where they were: what a side made is gone before the next one runs.
RESET = r'''<?php
global $wpdb;
$menus = get_option( 'rp_nav_menus', array() );
foreach ( wp_get_nav_menus() as $menu ) {
	if ( ! in_array( (int) $menu->term_id, $menus, true ) ) {
		wp_delete_nav_menu( $menu->term_id );
	}
}
$posts = (int) get_option( 'rp_nav_posts_max' );
$terms = (int) get_option( 'rp_nav_terms_max' );
$tts   = (int) get_option( 'rp_nav_tts_max' );
$wpdb->query( "DELETE FROM $wpdb->postmeta WHERE post_id > $posts" );
$wpdb->query( "DELETE FROM $wpdb->term_relationships WHERE object_id > $posts" );
$wpdb->query( "DELETE FROM $wpdb->posts WHERE ID > $posts" );
$wpdb->query( "DELETE FROM $wpdb->term_taxonomy WHERE term_taxonomy_id > $tts" );
$wpdb->query( "DELETE FROM $wpdb->terms WHERE term_id > $terms" );
$wpdb->query( 'ALTER TABLE ' . $wpdb->posts . ' AUTO_INCREMENT = ' . ( $posts + 1 ) );
$wpdb->query( 'ALTER TABLE ' . $wpdb->terms . ' AUTO_INCREMENT = ' . ( $terms + 1 ) );
$wpdb->query( 'ALTER TABLE ' . $wpdb->term_taxonomy . ' AUTO_INCREMENT = ' . ( $tts + 1 ) );
$saved = get_option( 'rp_nav_saved' );
update_option( 'theme_mods_twentyseventeen', $saved['mods'] );
update_option( 'nav_menu_options', $saved['options'], false );
foreach ( array( 'rpnav', 'rpnaved' ) as $login ) {
	$user = get_user_by( 'login', $login );
	foreach ( array( 'nav_menu_recently_edited', 'managenav-menuscolumnshidden', 'metaboxhidden_nav-menus' ) as $key ) {
		delete_user_meta( $user->ID, $key );
	}
}
wp_cache_flush();
'''

MARK = r'''<?php
global $wpdb;
update_option( 'rp_nav_menus', array_map( 'intval', wp_list_pluck( wp_get_nav_menus(), 'term_id' ) ), false );
update_option( 'rp_nav_posts_max', (int) $wpdb->get_var( "SELECT MAX(ID) FROM $wpdb->posts" ), false );
update_option( 'rp_nav_terms_max', (int) $wpdb->get_var( "SELECT MAX(term_id) FROM $wpdb->terms" ), false );
update_option( 'rp_nav_tts_max', (int) $wpdb->get_var( "SELECT MAX(term_taxonomy_id) FROM $wpdb->term_taxonomy" ), false );
'''

CLEANUP = r'''<?php
global $wpdb;
foreach ( array( 'rpnav-a', 'rpnav-b', 'rpnav-post' ) as $slug ) {
	$id = $wpdb->get_var( $wpdb->prepare( "SELECT ID FROM $wpdb->posts WHERE post_name = %s", $slug ) );
	if ( $id ) {
		wp_delete_post( $id, true );
	}
}
foreach ( array( 'rpnav-sub', 'rpnav-cat' ) as $slug ) {
	$term = get_term_by( 'slug', $slug, 'category' );
	if ( $term ) {
		wp_delete_term( $term->term_id, 'category' );
	}
}
require_once ABSPATH . 'wp-admin/includes/user.php';
foreach ( array( 'rpnav', 'rpnaved' ) as $login ) {
	$user = get_user_by( 'login', $login );
	if ( $user ) {
		wp_delete_user( $user->ID );
	}
}
$saved = get_option( 'rp_nav_saved' );
update_option( 'theme_mods_twentyseventeen', $saved['mods'] );
update_option( 'nav_menu_options', $saved['options'], false );
foreach ( array( 'rp_nav_saved', 'rp_nav_menus', 'rp_nav_posts_max', 'rp_nav_terms_max', 'rp_nav_tts_max' ) as $name ) {
	delete_option( $name );
}
'''

# What a step leaves behind: everything about menus, without the times.
SNAPSHOT = r'''<?php
global $wpdb;
$out = array();
$posts = (int) get_option( 'rp_nav_posts_max' );
foreach ( wp_get_nav_menus() as $menu ) {
	$out['menus'][] = array( $menu->term_id, $menu->name, $menu->slug, $menu->description, $menu->parent, (int) $wpdb->get_var( $wpdb->prepare( "SELECT count FROM $wpdb->term_taxonomy WHERE term_id = %d", $menu->term_id ) ) );
}
$rows = $wpdb->get_results( "SELECT ID, post_author, post_title, post_excerpt, post_content, post_name, post_status, post_parent, menu_order, guid, comment_status, ping_status FROM $wpdb->posts WHERE post_type = 'nav_menu_item' ORDER BY ID", ARRAY_A );
foreach ( $rows as $row ) {
	$row['meta']  = $wpdb->get_results( $wpdb->prepare( "SELECT meta_key, meta_value FROM $wpdb->postmeta WHERE post_id = %d AND meta_key != '_menu_item_orphaned' ORDER BY meta_key, meta_id", $row['ID'] ), ARRAY_N );
	$row['menus'] = $wpdb->get_col( $wpdb->prepare( "SELECT term_taxonomy_id FROM $wpdb->term_relationships WHERE object_id = %d ORDER BY term_taxonomy_id", $row['ID'] ) );
	$out['items'][] = $row;
}
$out['locations'] = get_theme_mod( 'nav_menu_locations' );
$out['options']   = $wpdb->get_var( "SELECT option_value FROM $wpdb->options WHERE option_name = 'nav_menu_options'" );
$out['autoload']  = $wpdb->get_var( "SELECT autoload FROM $wpdb->options WHERE option_name = 'nav_menu_options'" );
$user = get_user_by( 'login', 'rpnav' );
foreach ( array( 'nav_menu_recently_edited', 'managenav-menuscolumnshidden', 'metaboxhidden_nav-menus' ) as $key ) {
	$out['user'][ $key ] = $wpdb->get_var( $wpdb->prepare( "SELECT meta_value FROM $wpdb->usermeta WHERE user_id = %d AND meta_key = %s", $user->ID, $key ) );
}
echo json_encode( $out, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE );
'''


def norm(text, port):
    # From the body on: the copy is German, and the translations of core scripts in the head are open in PLAN.md.
    if not WHOLE:
        at = text.find('<body')
        localized = re.search(r'var menus = \{.*?\};', text)
        text = (localized.group(0) + '\n' if localized else '') + text[at if at >= 0 else 0:]
        end = text.find('<div id="wpfooter"')
        text = text[:end] if end >= 0 else text
    text = text.replace(f'localhost:{port}', 'localhost:PORT')
    text = re.sub(r'\b[0-9a-f]{10}\b', 'NONCE', text)
    text = re.sub(r'_wpnonce%3D[0-9a-f]{10}', '_wpnonce%3DNONCE', text)
    text = re.sub(r'"time":"\d+"', '"time":"T"', text)
    text = re.sub(r'cancel=\d+', 'cancel=T', text)
    return same_titles(text)


def same_titles(text):
    """Entries of a checklist with the same title come in the order MySQL leaves them (PLAN.md): sorted, as a run."""
    label = re.compile(r'name="menu-item\[-?\d+\]\[menu-item-object-id\]" value="\d+" /> ([^<]*)<')
    lines, out, run = text.split('\n'), [], []

    def flush():
        if len(run) > 1:
            cleared = [re.sub(r'menu-item\[-?\d+\]', 'menu-item[N]', line).strip() for line in run]
            out.extend(sorted(cleared))
        else:
            out.extend(run)
        run.clear()

    for line in lines:
        found = label.search(line)
        title = found.group(1).strip() if found else None
        if title is not None and run and label.search(run[-1]).group(1).strip() == title:
            run.append(line)
            continue
        flush()
        if title is not None:
            run.append(line)
        else:
            out.append(line)
    flush()
    return '\n'.join(out)


class Side:
    def __init__(self, port, user='rpnav'):
        self.port = port
        self.client = lc.Client(port)
        self.client.request('/wp-login.php')
        self.client.request('/wp-login.php', {'log': user, 'pwd': 'rustpress-dev', 'testcookie': '1'})
        self.nonces = {}
        self.ids = {}

    def get(self, page):
        status, headers, body = self.client.request('/wp-admin/' + page)
        self.remember(body)
        return status, headers, body

    def post(self, page, form):
        status, headers, body = self.client.request('/wp-admin/' + page, form)
        self.remember(body)
        return status, headers, body

    def remember(self, body):
        found = re.search(r'id="_wpnonce" name="_wpnonce" value="([0-9a-f]{10})"', body)
        if found:
            self.nonces['save-menu-locations'] = found.group(1)
        found = re.search(r'id="menu-settings-column-nonce" name="menu-settings-column-nonce" value="([0-9a-f]{10})"', body)
        if found:
            self.nonces['menu-settings-column-nonce'] = found.group(1)
        found = re.search(r'id="update-nav-menu-nonce" name="update-nav-menu-nonce" value="([0-9a-f]{10})"', body)
        if found:
            self.nonces['update-nav-menu-nonce'] = found.group(1)
        for action, nonce in re.findall(r'action=(move-up-menu-item|move-down-menu-item|delete)(?:&[^"\']*?)?_wpnonce=([0-9a-f]{10})', body.replace('&#038;', '&').replace('&amp;', '&')):
            self.nonces[action] = nonce
        for item, nonce in re.findall(r'action=delete-menu-item&menu-item=(\d+)&_wpnonce=([0-9a-f]{10})', body.replace('&#038;', '&').replace('&amp;', '&')):
            self.nonces[f'delete-item-{item}'] = nonce


def steps():
    """`(name, method, address, form, wanted)`. In forms and addresses `{name}` is a nonce a page gave, `@n` an ID."""
    yield ('real menu', 'GET', 'nav-menus.php', None, 'menu-item-settings')
    yield ('tab locations', 'GET', 'nav-menus.php?action=locations', None, 'menu-locations-table')
    yield ('new menu', 'GET', 'nav-menus.php?action=edit&menu=0', None, 'menu-name-desc')
    yield ('new menu for location', 'GET', 'nav-menus.php?action=edit&menu=0&use-location=social', None, 'use-location')
    # Writing.
    # What the page does with JavaScript.
    yield ('ajax metabox page', 'POST', 'admin-ajax.php', {'action': 'menu-get-metabox', 'item-type': 'post_type', 'item-object': 'page', 'page-tab': 'all', 'paged': '1'}, 'replace-id')
    yield ('ajax metabox category', 'POST', 'admin-ajax.php', {'action': 'menu-get-metabox', 'item-type': 'taxonomy', 'item-object': 'category', 'category-tab': 'all', 'paged': '1'}, 'replace-id')
    yield ('ajax search json', 'POST', 'admin-ajax.php', {'action': 'menu-quick-search', 'type': 'quick-search-posttype-page', 'q': 'rpnav', 'response-format': 'json'}, 'post_title')
    yield ('ajax search markup', 'POST', 'admin-ajax.php', {'action': 'menu-quick-search', 'type': 'quick-search-posttype-post', 'q': 'rpnav', 'response-format': 'markup'}, 'menu-item-checkbox')
    yield ('ajax search terms', 'POST', 'admin-ajax.php', {'action': 'menu-quick-search', 'type': 'quick-search-taxonomy-category', 'q': 'rpnav', 'response-format': 'markup'}, 'menu-item-checkbox')
    yield ('ajax item json', 'POST', 'admin-ajax.php', {'action': 'menu-quick-search', 'type': 'get-post-item', 'object_type': 'page', 'ID': '@page_a', 'response-format': 'json'}, 'post_title')
    yield ('create without name', 'POST', 'nav-menus.php', {'action': 'update', 'menu': '0', 'menu-name': '  ', 'update-nav-menu-nonce': '{update-nav-menu-nonce}'}, 'notice')
    yield ('create menu', 'POST', 'nav-menus.php', {'action': 'update', 'menu': '0', 'menu-name': 'RP Menü & Co', 'use-location': 'social', 'update-nav-menu-nonce': '{update-nav-menu-nonce}'}, '')
    yield ('after create', 'GET', 'nav-menus.php?menu=@menu', None, 'RP Men')
    yield ('name taken', 'POST', 'nav-menus.php', {'action': 'update', 'menu': '0', 'menu-name': 'RP Menü & Co', 'update-nav-menu-nonce': '{update-nav-menu-nonce}'}, 'notice')
    yield ('add custom link', 'POST', 'nav-menus.php?menu=@menu', {'action': 'add-menu-item', 'menu': '@menu', 'menu-settings-column-nonce': '{menu-settings-column-nonce}', 'menu-item[-1][menu-item-type]': 'custom', 'menu-item[-1][menu-item-url]': ' http://example.com/a?b=1&c=2 ', 'menu-item[-1][menu-item-title]': 'Link <b>eins</b>'}, 'pending')
    yield ('add page', 'POST', 'nav-menus.php?menu=@menu', {'action': 'add-menu-item', 'menu': '@menu', 'menu-settings-column-nonce': '{menu-settings-column-nonce}', 'menu-item[-2][menu-item-object-id]': '@page_a', 'menu-item[-2][menu-item-db-id]': '0', 'menu-item[-2][menu-item-object]': 'page', 'menu-item[-2][menu-item-parent-id]': '0', 'menu-item[-2][menu-item-type]': 'post_type', 'menu-item[-2][menu-item-title]': 'rpnav-a', 'menu-item[-2][menu-item-url]': 'http://localhost/x'}, 'pending')
    yield ('add category and post', 'POST', 'nav-menus.php?menu=@menu', {'action': 'add-menu-item', 'menu': '@menu', 'menu-settings-column-nonce': '{menu-settings-column-nonce}', 'menu-item[-3][menu-item-object-id]': '@cat', 'menu-item[-3][menu-item-db-id]': '0', 'menu-item[-3][menu-item-object]': 'category', 'menu-item[-3][menu-item-parent-id]': '0', 'menu-item[-3][menu-item-type]': 'taxonomy', 'menu-item[-3][menu-item-title]': 'rpnav-cat', 'menu-item[-4][menu-item-object-id]': '@post', 'menu-item[-4][menu-item-db-id]': '0', 'menu-item[-4][menu-item-object]': 'post', 'menu-item[-4][menu-item-parent-id]': '0', 'menu-item[-4][menu-item-type]': 'post_type', 'menu-item[-4][menu-item-title]': 'Eigener Titel'}, 'pending')
    yield ('save menu', 'POST', 'nav-menus.php?menu=@menu', {'action': 'update', 'menu': '@menu', 'menu-name': 'RP Menü neu', 'auto-add-pages': '1', 'menu-locations[top]': '@menu', 'update-nav-menu-nonce': '{update-nav-menu-nonce}',
                                                             'menu-item-db-id[@i1]': '@i1', 'menu-item-title[@i1]': 'Link <b>zwei</b>', 'menu-item-url[@i1]': 'http://example.com/z', 'menu-item-type[@i1]': 'custom', 'menu-item-object[@i1]': 'custom', 'menu-item-object-id[@i1]': '@i1', 'menu-item-parent-id[@i1]': '0', 'menu-item-position[@i1]': '1', 'menu-item-attr-title[@i1]': 'Attr "x"', 'menu-item-target[@i1]': '_blank', 'menu-item-classes[@i1]': 'eins zwei! drei', 'menu-item-xfn[@i1]': 'me friend', 'menu-item-description[@i1]': 'Text <i>x</i>',
                                                             'menu-item-db-id[@i2]': '@i2', 'menu-item-title[@i2]': 'rpnav-a', 'menu-item-type[@i2]': 'post_type', 'menu-item-object[@i2]': 'page', 'menu-item-object-id[@i2]': '@page_a', 'menu-item-parent-id[@i2]': '@i1', 'menu-item-position[@i2]': '2',
                                                             'menu-item-db-id[@i3]': '@i3', 'menu-item-title[@i3]': 'rpnav-cat', 'menu-item-type[@i3]': 'taxonomy', 'menu-item-object[@i3]': 'category', 'menu-item-object-id[@i3]': '@cat', 'menu-item-parent-id[@i3]': '0', 'menu-item-position[@i3]': '3',
                                                             'menu-item-db-id[@i4]': '@i4', 'menu-item-title[@i4]': 'Eigener Titel', 'menu-item-type[@i4]': 'post_type', 'menu-item-object[@i4]': 'post', 'menu-item-object-id[@i4]': '@post', 'menu-item-parent-id[@i4]': '0', 'menu-item-position[@i4]': '4'}, 'notice')
    yield ('edit item open', 'GET', 'nav-menus.php?menu=@menu&edit-menu-item=@i1', None, 'menu-item-edit-active')
    yield ('tab two menus', 'GET', 'nav-menus.php?action=locations', None, 'data-orig')
    yield ('move down', 'GET', 'nav-menus.php?action=move-down-menu-item&menu-item=@i3&_wpnonce={move-down-menu-item}', None, 'menu-item-settings')
    yield ('move up child', 'GET', 'nav-menus.php?action=move-up-menu-item&menu-item=@i2&_wpnonce={move-up-menu-item}', None, 'menu-item-settings')
    yield ('move up', 'GET', 'nav-menus.php?action=move-up-menu-item&menu-item=@i4&_wpnonce={move-up-menu-item}', None, 'menu-item-settings')
    yield ('move down last', 'GET', 'nav-menus.php?action=move-down-menu-item&menu-item=@i4&_wpnonce={move-down-menu-item}', None, 'menu-item-settings')
    yield ('delete item', 'GET', 'nav-menus.php?action=delete-menu-item&menu-item=@i4&_wpnonce={delete-item-@i4}', None, 'notice')
    yield ('remove by leaving out', 'POST', 'nav-menus.php?menu=@menu', {'action': 'update', 'menu': '@menu', 'menu-name': 'RP Menü neu', 'update-nav-menu-nonce': '{update-nav-menu-nonce}', 'menu-item-db-id[@i1]': '@i1', 'menu-item-title[@i1]': 'Nur noch eins', 'menu-item-url[@i1]': 'http://example.com/z', 'menu-item-type[@i1]': 'custom', 'menu-item-object[@i1]': 'custom', 'menu-item-object-id[@i1]': '@i1', 'menu-item-parent-id[@i1]': '0', 'menu-item-position[@i1]': '1'}, 'notice')
    yield ('assign locations', 'POST', 'nav-menus.php?action=locations', {'nav-menu-locations': 'x', 'menu-locations[top]': '@menu', 'menu-locations[social]': '0', '_wpnonce': '{save-menu-locations}'}, 'notice')
    yield ('ajax add items', 'POST', 'admin-ajax.php', {'action': 'add-menu-item', 'menu': '@menu', 'menu-settings-column-nonce': '{menu-settings-column-nonce}', 'menu-item[-1][menu-item-type]': 'custom', 'menu-item[-1][menu-item-url]': 'http://example.com/ajax', 'menu-item[-1][menu-item-title]': 'Ajax', 'menu-item[-2][menu-item-object-id]': '@cat', 'menu-item[-2][menu-item-db-id]': '0', 'menu-item[-2][menu-item-object]': 'category', 'menu-item[-2][menu-item-parent-id]': '0', 'menu-item[-2][menu-item-type]': 'taxonomy', 'menu-item[-2][menu-item-title]': 'rpnav-cat'}, 'menu-item-settings')
    yield ('ajax locations', 'POST', 'admin-ajax.php', {'action': 'menu-locations-save', 'menu-settings-column-nonce': '{menu-settings-column-nonce}', 'menu-locations[top]': '2', 'menu-locations[social]': '@menu'}, '1')
    yield ('bad nonce', 'POST', 'nav-menus.php', {'action': 'update', 'menu': '@menu', 'menu-name': 'x', 'update-nav-menu-nonce': '0000000000'}, '')
    yield ('delete menu', 'GET', 'nav-menus.php?action=delete&menu=@menu&_wpnonce={delete}', None, 'notice')


def substitute(value, side):
    for name in sorted(side.ids, key=len, reverse=True):
        value = value.replace('@' + name, str(side.ids[name]))
    for name, nonce in side.nonces.items():
        value = value.replace('{' + name + '}', nonce)
    return value


IDS = r'''<?php
global $wpdb;
$ids = array();
foreach ( array( 'page_a' => 'rpnav-a', 'post' => 'rpnav-post' ) as $key => $slug ) {
	$ids[ $key ] = (int) $wpdb->get_var( $wpdb->prepare( "SELECT ID FROM $wpdb->posts WHERE post_name = %s", $slug ) );
}
$ids['cat'] = (int) get_term_by( 'slug', 'rpnav-cat', 'category' )->term_id;
$known = get_option( 'rp_nav_menus', array() );
foreach ( wp_get_nav_menus() as $menu ) {
	if ( ! in_array( (int) $menu->term_id, $known, true ) ) {
		$ids['menu'] = (int) $menu->term_id;
	}
}
$i = 0;
foreach ( $wpdb->get_col( 'SELECT ID FROM ' . $wpdb->posts . ' WHERE post_type = "nav_menu_item" AND ID > ' . (int) get_option( 'rp_nav_posts_max' ) . ' ORDER BY ID' ) as $id ) {
	$ids[ 'i' . ++$i ] = (int) $id;
}
echo json_encode( $ids );
'''


def compare(name, wanted, responses, states, show):
    (status_a, head_a, body_a), (status_b, head_b, body_b) = responses
    a, b = norm(body_a, REFERENCE), norm(body_b, RUSTPRESS)
    pick = lambda head, port: {k.lower(): norm(v, port) for k, v in head.items() if k.lower() in HEADERS}
    head_a, head_b = pick(head_a, REFERENCE), pick(head_b, RUSTPRESS)
    counts = (a.count(wanted), b.count(wanted)) if wanted else ('-', '-')
    states = [state.replace(f'localhost:{port}', 'localhost:PORT') for state, port in zip(states, (REFERENCE, RUSTPRESS))]
    same = status_a == status_b and a == b and head_a == head_b and states[0] == states[1]
    vacuous = bool(wanted) and counts[0] == 0
    print(f"{'equal' if same else 'DIFF '} {name:30} {status_a} {status_b} {len(a):7} {len(b):7} {wanted!r}: {counts[0]}/{counts[1]}" + (' EMPTY' if vacuous else ''))
    if head_a != head_b:
        print('    headers', head_a, head_b)
    if show:
        lines = list(difflib.unified_diff(a.split('\n'), b.split('\n'), lineterm='', n=0))[2:]
        for index, line in enumerate(lines[:show]):
            other = lines[index + 1] if line.startswith('-') and index + 1 < len(lines) else lines[index - 1] if line.startswith('+') and index else ''
            at = next((i for i, (x, y) in enumerate(zip(line[1:], other[1:])) if x != y), 0)
            print('   ', line[0], line[1:][max(0, at - 60):at + 240])
        if len(lines) > show:
            print(f'    ... {len(lines) - show} more')
    if states[0] != states[1]:
        print('    ROWS DIFFER')
        for line in list(difflib.unified_diff(states[0].split('\n'), states[1].split('\n'), lineterm='', n=0))[2:14]:
            print('   ', line[:240])
    return same and not vacuous


def main():
    args = sys.argv[1:]
    show = int(option(args, '--show', '8'))
    global WHOLE
    WHOLE = '--whole' in args
    only = option(args, '--only', '')
    php(PREPARE)
    php(MARK)
    failed = total = 0
    try:
        results = []
        for port in (REFERENCE, RUSTPRESS):
            php(RESET)
            side = Side(port)
            run = []
            for name, method, page, form, wanted in steps():
                side.ids = json.loads(php(IDS))
                page = substitute(page, side)
                if method == 'GET':
                    response = side.get(page)
                else:
                    response = side.post(page, {substitute(k, side): substitute(v, side) for k, v in form.items()})
                run.append((name, wanted, response, php(SNAPSHOT)))
            results.append(run)
        # Who may not edit theme options.
        php(RESET)
        total += 1
        responses = [Side(port, 'rpnaved').get('nav-menus.php') for port in (REFERENCE, RUSTPRESS)]
        if not compare('editor', 'Du bist', responses, ('', ''), show):
            failed += 1
        for left, right in zip(*results):
            name, wanted = left[0], left[1]
            if only not in name:
                continue
            total += 1
            if not compare(name, wanted, (left[2], right[2]), (left[3], right[3]), show):
                failed += 1
    finally:
        php(RESET)
        php(CLEANUP)
    print(f'{total - failed} of {total} equal')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
