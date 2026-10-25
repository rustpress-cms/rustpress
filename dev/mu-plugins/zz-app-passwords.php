<?php
// Test-only: lets the reference WordPress offer application passwords over plain HTTP.
add_filter( 'wp_is_application_passwords_available', '__return_true' );
