<?php
// Template for dev/canonical_oracle.ps1: asks the real WordPress what
// redirect_canonical() decides for each URL. The list is substituted for the token below.
$_SERVER['HTTP_HOST'] = 'localhost:8080';
$_SERVER['REQUEST_METHOD'] = 'GET';
$home = 'http://localhost:8080';
$urls = [@@URLS@@];
// The reference runs on port 8080 and WordPress drops the port in the paged/feed
// branch, which defeats its own chained-redirect check. Compare without the port,
// as a default-port site would behave.
$strip = function ($x) { return $x ? str_replace(':8080', '', $x) : $x; };
foreach ($urls as $u) {
    $_SERVER['REQUEST_URI'] = $u;
    $parts = explode('?', $u, 2);
    $_GET = [];
    if (isset($parts[1])) parse_str($parts[1], $_GET);
    global $wp, $wp_query;
    $wp = new WP();
    $wp->main();
    $target = '';
    if (is_404() && '' !== get_query_var('name')) {
        // wp_old_slug_redirect() runs before redirect_canonical().
        $old = null;
        add_filter('wp_redirect', function ($l) use (&$old) { $old = $l; throw new Exception('redirect'); });
        ob_start();
        try { wp_old_slug_redirect(); } catch (Exception $e) {}
        ob_end_clean();
        remove_all_filters('wp_redirect');
        if ($old) $target = $old;
    }
    if ($target === '') {
        $r1 = redirect_canonical($home . $u, false);
        if ($r1 && $strip($r1) !== $strip($home . $u)) {
            $r2 = redirect_canonical(str_replace('http://localhost/', $home . '/', $r1), false);
            if (!$r2 || $strip($r2) === $strip($r1)) $target = $strip($r1);
        }
    }
    echo $u, "\t", str_replace('http://localhost', '', $target), "\n";
}

