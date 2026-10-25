<?php
/**
 * Plugin Name: RustPress test types
 * Description: Custom post types and a taxonomy, as a plugin would register them. Used by dev/seed_types.sh.
 */
add_action(
	'init',
	function () {
		register_post_type(
			'book',
			array(
				'label'        => 'Buecher',
				'public'       => true,
				'has_archive'  => true,
				'rewrite'      => array( 'slug' => 'buecher' ),
				'supports'     => array( 'title', 'editor', 'excerpt', 'thumbnail', 'comments', 'custom-fields', 'author' ),
				'show_in_rest' => true,
				'taxonomies'   => array( 'genre' ),
			)
		);
		register_post_type(
			'event',
			array(
				'label'        => 'Termine',
				'public'       => true,
				'has_archive'  => 'kalender',
				'rewrite'      => array( 'slug' => 'termine' ),
				'hierarchical' => true,
				'supports'     => array( 'title', 'editor', 'page-attributes' ),
				'show_in_rest' => true,
			)
		);
		register_post_type(
			'secret',
			array(
				'label'  => 'Geheim',
				'public' => false,
			)
		);
		register_taxonomy(
			'genre',
			array( 'book' ),
			array(
				'label'        => 'Genres',
				'public'       => true,
				'hierarchical' => true,
				'rewrite'      => array( 'slug' => 'genre' ),
				'show_in_rest' => true,
			)
		);
	}
);
