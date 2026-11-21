#!/usr/bin/env bash
# Compares Twenty Twenty-One with the reference in the settings the seeded content does not cover: a primary menu with a
# sub-menu and a description, a footer menu with social icons, a logo with the title hidden, a light and a dark custom
# background, the dark mode switch, full posts on the blog, a static front page with a posts page, and signed in.
#
#   dev/twentytwentyone-compare.sh [PATH ...]
#
# Needs the reference (dev/up.sh, seeded) and a built server (cargo build -p rustpress-server). Twenty Twenty-One is
# activated for the run; afterwards the menus, the theme mods and the user are removed and the previous theme and
# front page settings are restored. RustPress reads the site once, so it is started again on :3001 for every variant.
# Every page should be identical except the credit in the footer ("Proudly powered by RustPress").
set -euo pipefail
cd "$(dirname "$0")/.."
paths=("$@")
[ ${#paths[@]} -eq 0 ] && paths=(/ /page/2/ /hello-world/ /sample-page/ /diskussion/ /category/uncategorized/ "/?s=test" /nope/ /mehrseitig/2/ /2026/10/)
db=${RUSTPRESS_DATABASE_URL:-mysql://wordpress:wordpress@127.0.0.1:3306/wordpress}
root=${RUSTPRESS_WP_ROOT:-$(podman volume inspect rustpress-wp --format '{{.Mountpoint}}')}
sql() { podman exec rustpress-db mariadb -uwordpress -pwordpress wordpress -N -e "$1"; }
wp() { podman exec -i rustpress-wp php -- "$@"; }

php_helper=$(cat <<'PHP'
<?php
require '/var/www/html/wp-load.php';
require_once ABSPATH . 'wp-admin/includes/user.php';
$mode = $argv[1] ?? '';
if ( 'menus' === $mode ) {
	$primary = wp_create_nav_menu( 'TT1 Primary' );
	wp_update_nav_menu_item( $primary, 0, array( 'menu-item-title' => 'Start', 'menu-item-url' => home_url( '/' ), 'menu-item-status' => 'publish', 'menu-item-type' => 'custom' ) );
	$page   = get_page_by_path( 'sample-page' );
	$parent = wp_update_nav_menu_item( $primary, 0, array( 'menu-item-object-id' => $page->ID, 'menu-item-object' => 'page', 'menu-item-type' => 'post_type', 'menu-item-status' => 'publish', 'menu-item-description' => 'Eine Beschreibung' ) );
	wp_update_nav_menu_item( $primary, 0, array( 'menu-item-title' => 'Unterpunkt', 'menu-item-url' => home_url( '/hello-world/' ), 'menu-item-status' => 'publish', 'menu-item-type' => 'custom', 'menu-item-parent-id' => $parent ) );
	$category = get_category_by_slug( 'uncategorized' );
	wp_update_nav_menu_item( $primary, 0, array( 'menu-item-object-id' => $category->term_id, 'menu-item-object' => 'category', 'menu-item-type' => 'taxonomy', 'menu-item-status' => 'publish' ) );
	$footer = wp_create_nav_menu( 'TT1 Footer' );
	foreach ( array( 'https://github.com/rustpress-cms' => 'GitHub', 'https://mastodon.social/@x' => 'Mastodon', 'https://example.org/' => 'Beispiel' ) as $url => $title ) {
		wp_update_nav_menu_item( $footer, 0, array( 'menu-item-title' => $title, 'menu-item-url' => $url, 'menu-item-status' => 'publish', 'menu-item-type' => 'custom' ) );
	}
	set_theme_mod( 'nav_menu_locations', array( 'primary' => $primary, 'footer' => $footer ) );
	if ( ! get_user_by( 'login', 'rpadmin' ) ) {
		wp_insert_user( array( 'user_login' => 'rpadmin', 'user_pass' => 'rustpress-dev', 'role' => 'administrator', 'user_email' => 'rpadmin@example.org' ) );
		update_option( 'tt1_compare_user', 1 );
	}
} elseif ( 'mod' === $mode ) {
	set_theme_mod( $argv[2], json_decode( $argv[3], true ) );
} elseif ( 'unmod' === $mode ) {
	remove_theme_mod( $argv[2] );
} elseif ( 'logo' === $mode ) {
	$logo = get_posts( array( 'post_type' => 'attachment', 'post_mime_type' => 'image', 'posts_per_page' => 1, 'orderby' => 'ID', 'order' => 'ASC' ) );
	set_theme_mod( 'custom_logo', $logo ? $logo[0]->ID : 0 );
} elseif ( 'clean' === $mode ) {
	foreach ( array( 'TT1 Primary', 'TT1 Footer' ) as $name ) {
		$menu = wp_get_nav_menu_object( $name );
		if ( $menu ) {
			wp_delete_nav_menu( $menu->term_id );
		}
	}
	delete_option( 'theme_mods_twentytwentyone' );
	if ( get_option( 'tt1_compare_user' ) && ( $user = get_user_by( 'login', 'rpadmin' ) ) ) {
		wp_delete_user( $user->ID );
	}
	delete_option( 'tt1_compare_user' );
}
PHP
)
podman exec -i rustpress-wp sh -c 'cat > /tmp/tt1-compare.php' <<<"$php_helper"
helper() { podman exec rustpress-wp php /tmp/tt1-compare.php "$@" >/dev/null; }

saved=$(sql "select option_name, option_value from wp_options where option_name in ('template','stylesheet','show_on_front','page_on_front','page_for_posts','sidebars_widgets')")
restore() {
	helper clean || true
	while read -r name value; do
		sql "update wp_options set option_value='$value' where option_name='$name'"
	done <<<"$saved"
	[ -n "${server:-}" ] && kill "$server" 2>/dev/null || true
}
trap restore EXIT
sql "update wp_options set option_value='twentytwentyone' where option_name in ('template','stylesheet')"
helper menus

server=
start() {
	[ -n "$server" ] && kill "$server" 2>/dev/null && wait "$server" 2>/dev/null || true
	RUSTPRESS_DATABASE_URL=$db RUSTPRESS_WP_ROOT=$root RUSTPRESS_LISTEN=127.0.0.1:3001 RUSTPRESS_ALLOW_WRITES=1 target/debug/rustpress-server >/dev/null 2>&1 &
	server=$!
	for _ in $(seq 50); do curl -s -o /dev/null http://127.0.0.1:3001/ && return; sleep 0.2; done
}
compare() {
	echo "== $1"
	start
	python3 dev/front-compare.py --anon "${paths[@]}"
	python3 dev/front-compare.py "${paths[@]}"
}

compare "menus"
helper logo; helper mod display_title_and_tagline false; helper mod background_color '"eeeadd"'
helper mod respect_user_color_preference true; helper mod display_excerpt_or_full_post '"full"'
compare "logo without title, light background, dark mode switch, full posts"
helper unmod display_title_and_tagline; helper unmod display_excerpt_or_full_post; helper mod background_color '"1d2433"'
compare "logo with title, dark background"
front=$(sql "select ID from wp_posts where post_name='ueber-uns' and post_type='page'")
posts=$(sql "select ID from wp_posts where post_name='team' and post_type='page'")
sql "update wp_options set option_value='page' where option_name='show_on_front'; update wp_options set option_value='$front' where option_name='page_on_front'; update wp_options set option_value='$posts' where option_name='page_for_posts'"
paths=(/ /ueber-uns/team/ /ueber-uns/team/page/2/ /hello-world/)
compare "static front page with a posts page"
