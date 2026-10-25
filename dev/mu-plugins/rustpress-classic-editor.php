<?php
/**
 * Plugin Name: RustPress classic editor
 * Description: Uses the classic edit form instead of the block editor in the reference WordPress, so that the
 *              classic form of RustPress can be compared with it.
 */
add_filter( 'use_block_editor_for_post', '__return_false', 100 );
add_filter( 'use_block_editor_for_post_type', '__return_false', 100 );
