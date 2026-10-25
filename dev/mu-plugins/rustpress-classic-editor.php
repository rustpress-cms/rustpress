<?php
/**
 * Plugin Name: RustPress classic editor
 * Description: Uses the classic edit form instead of the block editor in the reference WordPress when the request
 *              has the parameter `rp-classic`, so that the classic form of RustPress can be compared with it.
 */
if ( isset( $_GET['rp-classic'] ) ) {
	add_filter( 'use_block_editor_for_post', '__return_false', 100 );
	add_filter( 'use_block_editor_for_post_type', '__return_false', 100 );
}
