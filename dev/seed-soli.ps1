# Adds widgets and comments to the copy of the real site that dev/up-soli.sh serves (database
# `soli_ref`), so that the parts of Twenty Seventeen the site itself does not use can be compared
# with real WordPress as well. Only the scratch copy is changed.
#   dev/seed-soli.ps1
$ErrorActionPreference = 'Stop'

function Php($v) {
    if ($v -is [bool]) { return 'b:' + [int]$v + ';' }
    if ($v -is [int]) { return "i:$v;" }
    if ($v -is [string]) { return 's:' + [Text.Encoding]::UTF8.GetByteCount($v) + ':"' + $v + '";' }
    if ($v -is [System.Collections.IDictionary]) {
        $out = 'a:' + @($v.Keys).Count + ':{'
        foreach ($k in @($v.Keys)) { $key = if ("$k" -match '^\d+$') { "i:$k;" } else { Php "$k" }; $out += $key + (Php $v["$k"]) }
        return $out + '}'
    }
    throw "unsupported $v"
}
function Sql($v) { (Php $v).Replace("\", "\\").Replace("'", "''") }

$widgets = [ordered]@{
    'widget_search'          = [ordered]@{ '2' = [ordered]@{ title = 'Suchen' }; '_multiwidget' = 1 }
    'widget_archives'        = [ordered]@{ '2' = [ordered]@{ title = ''; count = 1; dropdown = 0 }; '_multiwidget' = 1 }
    'widget_categories'      = [ordered]@{ '2' = [ordered]@{ title = ''; count = 1; hierarchical = 1; dropdown = 0 }; '_multiwidget' = 1 }
    'widget_meta'            = [ordered]@{ '2' = [ordered]@{ title = '' }; '_multiwidget' = 1 }
    'widget_recent-comments' = [ordered]@{ '2' = [ordered]@{ title = ''; number = 5 }; '_multiwidget' = 1 }
    'widget_text'            = [ordered]@{ '3' = [ordered]@{ title = 'Info'; text = '<p>Hallo <strong>Welt</strong> & Co.</p>'; filter = $true; visual = $true }; '_multiwidget' = 1 }
    'widget_pages'           = [ordered]@{ '2' = [ordered]@{ title = ''; sortby = 'post_title'; exclude = '' }; '_multiwidget' = 1 }
    'widget_tag_cloud'       = [ordered]@{ '2' = [ordered]@{ title = ''; taxonomy = 'post_tag' }; '_multiwidget' = 1 }
    'widget_calendar'        = [ordered]@{ '2' = [ordered]@{ title = '' }; '_multiwidget' = 1 }
    'widget_block'           = [ordered]@{ '2' = [ordered]@{ content = "<!-- wp:paragraph -->`n<p>Ein Block im Widget.</p>`n<!-- /wp:paragraph -->" }; '_multiwidget' = 1 }
}
$sidebars = [ordered]@{
    'wp_inactive_widgets' = @()
    'sidebar-1' = @('search-2', 'recent-posts-6', 'archives-2', 'categories-2', 'meta-2', 'text-3', 'nav_menu-3', 'recent-comments-2')
    'sidebar-2' = @('pages-2', 'tag_cloud-2')
    'sidebar-3' = @('block-2', 'calendar-2')
    'array_version' = 3
}
function SidebarPhp($s) {
    $out = 'a:' + $s.Count + ':{'
    foreach ($k in $s.Keys) {
        $out += (Php $k)
        if ($s[$k] -is [int]) { $out += "i:$($s[$k]);" }
        else {
            $list = @($s[$k]); $out += 'a:' + $list.Count + ':{'
            for ($i = 0; $i -lt $list.Count; $i++) { $out += "i:$i;" + (Php $list[$i]) }
            $out += '}'
        }
    }
    return $out + '}'
}

$sql = New-Object Text.StringBuilder
foreach ($name in $widgets.Keys) {
    [void]$sql.AppendLine("REPLACE INTO wp_options (option_name, option_value, autoload) VALUES ('$name', '$(Sql $widgets[$name])', 'yes');")
}
[void]$sql.AppendLine("REPLACE INTO wp_options (option_name, option_value, autoload) VALUES ('sidebars_widgets', '$((SidebarPhp $sidebars).Replace("'", "''"))', 'yes');")
# Comments: one thread on post 61, one more on the page 41, and one that is not approved.
[void]$sql.AppendLine(@"
DELETE FROM wp_comments WHERE comment_ID >= 9000;
UPDATE wp_posts SET comment_status = 'open', ping_status = 'open' WHERE ID IN (61, 41);
INSERT INTO wp_comments (comment_ID, comment_post_ID, comment_author, comment_author_email, comment_author_url, comment_author_IP, comment_date, comment_date_gmt, comment_content, comment_karma, comment_approved, comment_agent, comment_type, comment_parent, user_id) VALUES
 (9001, 61, 'Anna', 'anna@example.com', 'http://example.org', '127.0.0.1', '2026-10-01 10:00:00', '2026-10-01 08:00:00', 'Erster Kommentar mit "Anführungszeichen" und einem Absatz.\n\nZweiter Absatz.', 0, '1', 'test', 'comment', 0, 0),
 (9002, 61, 'Bernd', 'bernd@example.com', '', '127.0.0.1', '2026-10-02 11:30:00', '2026-10-02 09:30:00', 'Antwort auf den ersten Kommentar.', 0, '1', 'test', 'comment', 9001, 0),
 (9003, 61, '', '', '', '127.0.0.1', '2026-10-03 12:00:00', '2026-10-03 10:00:00', 'Anonymer Kommentar', 0, '1', 'test', 'comment', 0, 0),
 (9004, 61, 'Carla', 'carla@example.com', '', '127.0.0.1', '2026-10-04 12:00:00', '2026-10-04 10:00:00', 'Noch nicht freigegeben', 0, '0', 'test', 'comment', 0, 0),
 (9005, 41, 'Dora', 'dora@example.com', 'https://dora.example', '127.0.0.1', '2026-10-05 12:00:00', '2026-10-05 10:00:00', 'Kommentar auf einer Seite', 0, '1', 'test', 'comment', 0, 0);
UPDATE wp_posts SET comment_count = (SELECT COUNT(*) FROM wp_comments c WHERE c.comment_post_ID = wp_posts.ID AND c.comment_approved = '1') WHERE ID IN (61, 41);
"@)
$file = Join-Path $env:TEMP 'seed-soli.sql'
[IO.File]::WriteAllText($file, $sql.ToString(), (New-Object Text.UTF8Encoding($false)))
cmd /c "wsl.exe -d Debian -- bash -c ""podman exec -i rustpress-db mariadb -uroot -prustpress --default-character-set=utf8mb4 soli_ref"" < $file"
Write-Output 'seeded soli_ref'
