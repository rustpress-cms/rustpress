#!/usr/bin/env bash
# Compares the list screens of the reference and RustPress for a matrix of query arguments (body only, via dev/admin-compare.sh).
cd "$(dirname "$0")/.."
screens=(
 "edit.php?post_status=draft" "edit.php?post_status=publish" "edit.php?post_status=trash" "edit.php?post_status=future" "edit.php?post_status=private" "edit.php?post_status=pending" "edit.php?post_status=all"
 "edit.php?orderby=title&order=asc" "edit.php?orderby=title&order=desc" "edit.php?orderby=date&order=asc" "edit.php?orderby=comment_count&order=desc" "edit.php?orderby=author&order=asc"
 "edit.php?s=hello" "edit.php?s=zzzznomatch" "edit.php?cat=1" "edit.php?author=1" "edit.php?author=3" "edit.php?m=202610" "edit.php?m=201501" "edit.php?paged=2" "edit.php?post_format=standard" "edit.php?tag=nope" "edit.php?post_status=publish&orderby=title"
 "edit.php?post_type=page" "edit.php?post_type=page&post_status=draft" "edit.php?post_type=page&orderby=title&order=asc" "edit.php?post_type=page&s=a" "edit.php?post_type=book" "edit.php?post_type=event" "edit.php?post_type=nope"
 "upload.php" "upload.php?mode=list" "upload.php?mode=grid" "upload.php?attachment-filter=post_mime_type:image" "upload.php?attachment-filter=detached" "upload.php?attachment-filter=mine" "upload.php?m=202610" "upload.php?s=test" "upload.php?orderby=title&order=asc" "upload.php?orderby=date&order=asc" "upload.php?post_mime_type=image&mode=list"
 "edit-comments.php?comment_status=all" "edit-comments.php?comment_status=moderated" "edit-comments.php?comment_status=approved" "edit-comments.php?comment_status=spam" "edit-comments.php?comment_status=trash" "edit-comments.php?comment_type=comment" "edit-comments.php?s=a" "edit-comments.php?orderby=comment_date&order=asc" "edit-comments.php?p=1" "edit-comments.php?comment_type=pings"
 "users.php?role=administrator" "users.php?role=editor" "users.php?role=none" "users.php?s=rp" "users.php?orderby=login&order=desc" "users.php?orderby=email&order=asc" "users.php?orderby=registered&order=asc" "users.php?orderby=name"
 "edit-tags.php?taxonomy=category&s=a" "edit-tags.php?taxonomy=category&orderby=count&order=desc" "edit-tags.php?taxonomy=category&orderby=name&order=desc" "edit-tags.php?taxonomy=post_tag" "edit-tags.php?taxonomy=post_tag&orderby=count" "edit-tags.php?taxonomy=genre&post_type=book" "edit-tags.php?taxonomy=post_format"
 "plugins.php?plugin_status=active" "plugins.php?plugin_status=inactive" "plugins.php?plugin_status=mustuse" "plugins.php?plugin_status=recently_activated" "plugins.php?plugin_status=upgrade" "plugins.php?plugin_status=dropins" "plugins.php?s=akis"
 "themes.php?search=twenty" "themes.php?theme=twentytwentyfour"
 "options-general.php" "options-writing.php" "options-reading.php" "options-discussion.php" "options-media.php" "options-permalink.php" "options-privacy.php" "options-privacy.php?tab=policyguide"
 "tools.php" "import.php" "export.php" "export.php?content=posts" "site-health.php" "site-health.php?tab=debug" "update-core.php" "profile.php" "user-new.php" "user-edit.php?user_id=1" "user-edit.php?user_id=3" "media-new.php" "post-new.php" "post-new.php?post_type=page" "post-new.php?post_type=book" "link-manager.php" "nav-menus.php" "widgets.php" "customize.php" "theme-editor.php" "plugin-editor.php" "export-personal-data.php" "erase-personal-data.php" "about.php" "credits.php" "freedoms.php" "contribute.php" "authorize-application.php" "admin.php?page=nope" "nope.php" "index.php" "index.php?welcome=1"
)
bash dev/admin-compare.sh "${screens[@]}" | grep -v '^identical'
