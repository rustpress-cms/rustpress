#!/usr/bin/env bash
# Error pages and permission failures: the same addresses on the reference and RustPress, as rpadmin and as rpeditor.
cd "$(dirname "$0")/.."
admin=(
 "edit.php?post_type=nope" "post-new.php?post_type=nope" "edit-tags.php?taxonomy=nope" "edit-tags.php?taxonomy=post_format" "edit-tags.php" "term.php?taxonomy=nope&tag_ID=1" "term.php?taxonomy=category&tag_ID=99999"
 "post.php?post=999999&action=edit" "post.php?post=1&action=nope" "post.php" "post.php?action=edit" "comment.php?action=editcomment&c=99999" "comment.php" "user-edit.php?user_id=99999" "user-edit.php"
 "plugin-install.php?tab=nope" "theme-install.php?tab=nope" "admin.php?page=nope" "admin.php" "options.php" "upgrade.php" "nope.php" "wp-admin" "revision.php" "revision.php?revision=999999" "upload.php?mode=nope"
 "media.php" "media-upload.php" "async-upload.php" "ms-admin.php" "load-styles.php" "network/" "install.php" "setup-config.php" "menu.php" "admin-header.php" "update.php" "update.php?action=nope"
 "export.php?download=true" "import.php?import=nope" "themes.php?action=nope" "plugins.php?action=nope" "users.php?action=nope" "edit.php?action=nope" "edit-comments.php?action=nope"
)
editor=(
 "index.php" "options-general.php" "options-writing.php" "options-reading.php" "options-media.php" "options-permalink.php" "options-privacy.php" "plugins.php" "plugin-install.php" "plugin-editor.php" "themes.php" "theme-install.php" "theme-editor.php" "site-editor.php" "font-library.php" "options-connectors.php"
 "users.php" "user-new.php" "user-edit.php?user_id=1" "user-edit.php?user_id=4" "profile.php" "tools.php" "import.php" "export.php" "site-health.php" "update-core.php" "edit-tags.php?taxonomy=category" "edit-tags.php?taxonomy=post_tag" "term.php?taxonomy=category&tag_ID=1"
 "edit.php" "edit.php?post_type=page" "post-new.php" "post.php?post=1&action=edit" "post.php?post=2&action=edit" "upload.php" "media-new.php" "edit-comments.php" "comment.php?action=editcomment&c=1" "revision.php?revision=308" "authorize-application.php" "about.php" "export-personal-data.php" "erase-personal-data.php" "privacy-policy-guide.php" "options.php"
)
echo "== as rpadmin"; ADMIN_USER=rpadmin bash dev/admin-compare.sh "${admin[@]}" | grep -v '^identical'
echo "== as rpeditor"; ADMIN_USER=rpeditor bash dev/admin-compare.sh "${editor[@]}" | grep -v '^identical'
