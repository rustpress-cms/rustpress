<?php
/**
 * Plugin Name: RustPress pinned reference
 * Description: Keeps the reference WordPress at the release in WORDPRESS_VERSION (dev/wp-reference.sh): neither the
 *              core nor its translations update themselves. Unlike AUTOMATIC_UPDATER_DISABLED this leaves the
 *              auto-update interface as it is on any site, so the administration can be compared.
 */
add_filter( 'auto_update_core', '__return_false' );
add_filter( 'auto_update_translation', '__return_false' );
