
<div class="wrap">
<h1 class="wp-heading-inline">
Edit Media</h1>

 <a href="{{SITE}}/wp-admin/media-new.php" class="page-title-action">Add Media File</a>
<hr class="wp-header-end">

{{MESSAGE}}<div id="lost-connection-notice" class="notice error hidden"><p><span class="spinner"></span> <strong>Connection lost.</strong> Saving has been disabled until you are reconnected. <span class="hide-if-no-sessionstorage">This post is being backed up in your browser, just in case.</span></p></div><form name="post" action="post.php" method="post" id="post"
>
<input type="hidden" id="_wpnonce" name="_wpnonce" value="{{NONCE}}" /><input type="hidden" name="_wp_http_referer" value="{{REFERER}}" /><input type="hidden" id="user-id" name="user_ID" value="{{USER}}" />
<input type="hidden" id="hiddenaction" name="action" value="editpost" />
<input type="hidden" id="originalaction" name="originalaction" value="editpost" />
<input type="hidden" id="post_author" name="post_author" value="{{POST_AUTHOR}}" />
<input type="hidden" id="post_type" name="post_type" value="attachment" />
<input type="hidden" id="original_post_status" name="original_post_status" value="inherit" />
<input type="hidden" id="referredby" name="referredby" value="" />
<input type="hidden" id="active_post_lock" value="{{LOCK}}" />
	<input type="hidden" name="_wp_original_http_referer" value="" /><input type='hidden' id='post_ID' name='post_ID' value='{{ID}}' /><input type="hidden" id="meta-box-order-nonce" name="meta-box-order-nonce" value="{{NONCE_ORDER}}" /><input type="hidden" id="closedpostboxesnonce" name="closedpostboxesnonce" value="{{NONCE_CLOSED}}" />

<div id="poststuff">
<div id="post-body" class="metabox-holder columns-2">
<div id="post-body-content">

<div id="titlediv">
<div id="titlewrap">
		<label class="screen-reader-text" id="title-prompt-text" for="title">Add title</label>
	<input type="text" name="post_title" size="30" value="{{TITLE}}" id="title" spellcheck="true" autocomplete="off" />
	</div>
	<div class="inside">
		<div id="edit-slug-box" class="hide-if-no-js">
			<strong>Permalink:</strong>
{{SAMPLE_PERMALINK}}
	</div>
			</div>
	<input type="hidden" id="samplepermalinknonce" name="samplepermalinknonce" value="{{NONCE_SAMPLE}}" /></div><!-- /titlediv -->
		<div class="wp_attachment_holder wp-clearfix">
{{HOLDER}}			</div>
	<div class="wp_attachment_details edit-form-section">
	{{ALT_BLOCK}}
		<p>
			<label for="attachment_caption"><strong>
				{{CAPTION_LABEL}}			</strong></label><br />
			<textarea class="widefat" name="excerpt" id="attachment_caption">{{CAPTION}}</textarea>
		</p>

	
	<label for="attachment_content" class="attachment-content-description"><strong>Description</strong>
		</label>
	<div id="wp-attachment_content-wrap" class="wp-core-ui wp-editor-wrap html-active"><link rel='stylesheet' id='editor-buttons-css' href='{{SITE}}/wp-includes/css/editor.min.css?ver={{VERSION}}' media='all' />
<div id="wp-attachment_content-editor-container" class="wp-editor-container"><div id="qt_attachment_content_toolbar" class="quicktags-toolbar hide-if-no-js"></div><textarea class="wp-editor-area" rows="5" cols="40" name="content" id="attachment_content">{{DESCRIPTION}}</textarea></div>
</div>


	</div>
	<input type="hidden" id="image-edit-context" value="edit-attachment" />
</div><!-- /post-body-content -->

<div id="postbox-container-1" class="postbox-container">
<div id="side-sortables" class="meta-box-sortables"><div id="submitdiv" class="postbox "  role="region" aria-label="Save">
<div class="postbox-header"><h2 class="hndle" id="submitdiv-title">Save</h2>
<div class="handle-actions hide-if-no-js"><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move up" type="button" class="handle-order-higher wp-tooltip__toggle" aria-describedby="submitdiv-title">
							<span class="screen-reader-text">Move up</span>
							<span class="order-higher-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="submitdiv-handle-order-higher-description" class="wp-tooltip__bubble" role="tooltip"><span id="submitdiv-handle-order-higher-description-text" class="wp-tooltip__text">Move up</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move down" type="button" class="handle-order-lower wp-tooltip__toggle" aria-describedby="submitdiv-title">
							<span class="screen-reader-text">Move down</span>
							<span class="order-lower-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="submitdiv-handle-order-lower-description" class="wp-tooltip__bubble" role="tooltip"><span id="submitdiv-handle-order-lower-description-text" class="wp-tooltip__text">Move down</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Show or hide panel" type="button" class="handlediv wp-tooltip__toggle" aria-expanded="true" aria-describedby="submitdiv-title">
							<span class="screen-reader-text">Show or hide panel</span>
							<span class="toggle-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="submitdiv-handlediv" class="wp-tooltip__bubble" role="tooltip"><span id="submitdiv-handlediv-text" class="wp-tooltip__text">Show or hide panel</span></span></span></div></div><div class="inside">
<div class="submitbox" id="submitpost">

<div id="minor-publishing">

	<div style="display:none;">
	<p class="submit"><input type="submit" name="save" id="save" class="button" value="Save"  /></p></div>


<div id="misc-publishing-actions">
	<div class="misc-pub-section curtime misc-pub-curtime">
		<span id="timestamp">
			Uploaded on: <b>{{UPLOADED}}</b>		</span>
	</div><!-- .misc-pub-section -->

		<div class="misc-pub-section misc-pub-uploadedby word-wrap-break-word">
{{UPLOADED_BY}}			</div>

{{UPLOADED_TO}}
	<div class="misc-pub-section misc-pub-attachment">
		<label for="attachment_url">File URL:</label>
		<input type="text" class="widefat urlfield ltr" readonly="readonly" name="attachment_url" id="attachment_url" value="{{URL}}" />
		<span class="copy-to-clipboard-container">
			<button type="button" class="button copy-attachment-url edit-media" data-clipboard-target="#attachment_url">Copy URL to clipboard</button>
			<span class="success hidden" aria-hidden="true">Copied!</span>
		</span>
	</div>
	<div class="misc-pub-section misc-pub-download">
		<a href="{{URL}}" download>Download file</a>
	</div>
	<div class="misc-pub-section misc-pub-filename">
		File name: <strong>{{FILENAME}}</strong>
	</div>
	<div class="misc-pub-section misc-pub-filetype">
		File type:		<strong>
		{{FILETYPE}}		</strong>
	</div>

{{FILESIZE_BLOCK}}{{DIMS_BLOCK}}{{ORIGINAL_BLOCK}}		</div><!-- #misc-publishing-actions -->
<div class="clear"></div>
</div><!-- #minor-publishing -->

<div id="major-publishing-actions">
	<div id="delete-action">
	<a class="submitdelete deletion" onclick='return showNotice.warn();' href="{{DELETE_URL}}">Delete permanently</a>	</div>

	<div id="publishing-action">
		<span class="spinner"></span>
		<input name="original_publish" type="hidden" id="original_publish" value="Update" />
		<input name="save" type="submit" class="button button-primary button-large" id="publish" value="Update" />
	</div>
	<div class="clear"></div>
</div><!-- #major-publishing-actions -->

</div>

	</div>
</div>
</div></div>
<div id="postbox-container-2" class="postbox-container">
<div id="normal-sortables" class="meta-box-sortables"><div id="commentstatusdiv" class="postbox  hide-if-js"  role="region" aria-label="Discussion">
<div class="postbox-header"><h2 class="hndle" id="commentstatusdiv-title">Discussion</h2>
<div class="handle-actions hide-if-no-js"><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move up" type="button" class="handle-order-higher wp-tooltip__toggle" aria-describedby="commentstatusdiv-title">
							<span class="screen-reader-text">Move up</span>
							<span class="order-higher-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="commentstatusdiv-handle-order-higher-description" class="wp-tooltip__bubble" role="tooltip"><span id="commentstatusdiv-handle-order-higher-description-text" class="wp-tooltip__text">Move up</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move down" type="button" class="handle-order-lower wp-tooltip__toggle" aria-describedby="commentstatusdiv-title">
							<span class="screen-reader-text">Move down</span>
							<span class="order-lower-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="commentstatusdiv-handle-order-lower-description" class="wp-tooltip__bubble" role="tooltip"><span id="commentstatusdiv-handle-order-lower-description-text" class="wp-tooltip__text">Move down</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Show or hide panel" type="button" class="handlediv wp-tooltip__toggle" aria-expanded="true" aria-describedby="commentstatusdiv-title">
							<span class="screen-reader-text">Show or hide panel</span>
							<span class="toggle-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="commentstatusdiv-handlediv" class="wp-tooltip__bubble" role="tooltip"><span id="commentstatusdiv-handlediv-text" class="wp-tooltip__text">Show or hide panel</span></span></span></div></div><div class="inside">
<input name="advanced_view" type="hidden" value="1" />
<p class="meta-options">
	<label for="comment_status" class="selectit"><input name="comment_status" type="checkbox" id="comment_status" value="open" {{COMMENT_STATUS}}/> Allow comments</label><br />
	<label for="ping_status" class="selectit"><input name="ping_status" type="checkbox" id="ping_status" value="open" {{PING_STATUS}}/>
		Allow <a href="https://wordpress.org/documentation/article/introduction-to-blogging/#managing-comments">trackbacks and pingbacks</a>	</label>
	</p>
	</div>
</div>
<div id="commentsdiv" class="postbox  hide-if-js"  role="region" aria-label="Comments">
<div class="postbox-header"><h2 class="hndle" id="commentsdiv-title">Comments</h2>
<div class="handle-actions hide-if-no-js"><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move up" type="button" class="handle-order-higher wp-tooltip__toggle" aria-describedby="commentsdiv-title">
							<span class="screen-reader-text">Move up</span>
							<span class="order-higher-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="commentsdiv-handle-order-higher-description" class="wp-tooltip__bubble" role="tooltip"><span id="commentsdiv-handle-order-higher-description-text" class="wp-tooltip__text">Move up</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move down" type="button" class="handle-order-lower wp-tooltip__toggle" aria-describedby="commentsdiv-title">
							<span class="screen-reader-text">Move down</span>
							<span class="order-lower-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="commentsdiv-handle-order-lower-description" class="wp-tooltip__bubble" role="tooltip"><span id="commentsdiv-handle-order-lower-description-text" class="wp-tooltip__text">Move down</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Show or hide panel" type="button" class="handlediv wp-tooltip__toggle" aria-expanded="true" aria-describedby="commentsdiv-title">
							<span class="screen-reader-text">Show or hide panel</span>
							<span class="toggle-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="commentsdiv-handlediv" class="wp-tooltip__bubble" role="tooltip"><span id="commentsdiv-handlediv-text" class="wp-tooltip__text">Show or hide panel</span></span></span></div></div><div class="inside">
<input type="hidden" id="add_comment_nonce" name="add_comment_nonce" value="{{NONCE_ADD_COMMENT}}" />	<p class="hide-if-no-js" id="add-new-comment"><button type="button" class="button" onclick="window.commentReply && commentReply.addcomment({{ID}});">Add Comment</button></p>
	<input type="hidden" id="_ajax_fetch_list_nonce" name="_ajax_fetch_list_nonce" value="{{NONCE_FETCH_LIST}}" /><input type="hidden" name="_wp_http_referer" value="{{REFERER}}" /><table class="widefat fixed striped table-view-list comments wp-list-table comments-box" style="display:none;">
	<tbody id="the-comment-list"
		 data-wp-lists='list:comment'		>
			</tbody>
</table>
		<p id="no-comments">No comments yet.</p><div class="hidden" id="trash-undo-holder">
	<div class="trash-undo-inside">
		Comment by <strong></strong> moved to the Trash.		<span class="undo untrash"><a href="#">Undo</a></span>
	</div>
</div>
<div class="hidden" id="spam-undo-holder">
	<div class="spam-undo-inside">
		Comment by <strong></strong> marked as spam.		<span class="undo unspam"><a href="#">Undo</a></span>
	</div>
</div>
	</div>
</div>
<div id="slugdiv" class="postbox  hide-if-js"  role="region" aria-label="Slug">
<div class="postbox-header"><h2 class="hndle" id="slugdiv-title">Slug</h2>
<div class="handle-actions hide-if-no-js"><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move up" type="button" class="handle-order-higher wp-tooltip__toggle" aria-describedby="slugdiv-title">
							<span class="screen-reader-text">Move up</span>
							<span class="order-higher-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="slugdiv-handle-order-higher-description" class="wp-tooltip__bubble" role="tooltip"><span id="slugdiv-handle-order-higher-description-text" class="wp-tooltip__text">Move up</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move down" type="button" class="handle-order-lower wp-tooltip__toggle" aria-describedby="slugdiv-title">
							<span class="screen-reader-text">Move down</span>
							<span class="order-lower-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="slugdiv-handle-order-lower-description" class="wp-tooltip__bubble" role="tooltip"><span id="slugdiv-handle-order-lower-description-text" class="wp-tooltip__text">Move down</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Show or hide panel" type="button" class="handlediv wp-tooltip__toggle" aria-expanded="true" aria-describedby="slugdiv-title">
							<span class="screen-reader-text">Show or hide panel</span>
							<span class="toggle-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="slugdiv-handlediv" class="wp-tooltip__bubble" role="tooltip"><span id="slugdiv-handlediv-text" class="wp-tooltip__text">Show or hide panel</span></span></span></div></div><div class="inside">
<label class="screen-reader-text" for="post_name">
	Slug</label><input name="post_name" type="text" class="large-text" id="post_name" value="{{SLUG}}" />
	</div>
</div>
<div id="authordiv" class="postbox  hide-if-js"  role="region" aria-label="Author">
<div class="postbox-header"><h2 class="hndle" id="authordiv-title">Author</h2>
<div class="handle-actions hide-if-no-js"><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move up" type="button" class="handle-order-higher wp-tooltip__toggle" aria-describedby="authordiv-title">
							<span class="screen-reader-text">Move up</span>
							<span class="order-higher-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="authordiv-handle-order-higher-description" class="wp-tooltip__bubble" role="tooltip"><span id="authordiv-handle-order-higher-description-text" class="wp-tooltip__text">Move up</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Move down" type="button" class="handle-order-lower wp-tooltip__toggle" aria-describedby="authordiv-title">
							<span class="screen-reader-text">Move down</span>
							<span class="order-lower-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="authordiv-handle-order-lower-description" class="wp-tooltip__bubble" role="tooltip"><span id="authordiv-handle-order-lower-description-text" class="wp-tooltip__text">Move down</span></span></span><span class="wp-tooltip wp-is-tooltip">
				<button aria-label="Show or hide panel" type="button" class="handlediv wp-tooltip__toggle" aria-expanded="true" aria-describedby="authordiv-title">
							<span class="screen-reader-text">Show or hide panel</span>
							<span class="toggle-indicator" aria-hidden="true"></span>
						</button>
				<span popover="hint" id="authordiv-handlediv" class="wp-tooltip__bubble" role="tooltip"><span id="authordiv-handlediv-text" class="wp-tooltip__text">Show or hide panel</span></span></span></div></div><div class="inside">
<label class="screen-reader-text" for="post_author_override">
	Author</label>
	<select name='post_author_override' id='post_author_override' class=''>
{{AUTHORS}}</select></div>
</div>
</div><div id="advanced-sortables" class="meta-box-sortables"></div></div>
</div><!-- /post-body -->
<br class="clear" />
</div><!-- /poststuff -->
</form>
</div>

<form method="get">
	<table style="display:none;"><tbody id="com-reply"><tr id="replyrow" class="inline-edit-row" style="display:none;"><td colspan="2" class="colspanchange">
	<fieldset class="comment-reply">
	<legend>
		<span class="hidden" id="editlegend">Edit Comment</span>
		<span class="hidden" id="replyhead">Reply to Comment</span>
		<span class="hidden" id="addhead">Add Comment</span>
	</legend>

	<div id="replycontainer">
	<label for="replycontent" class="screen-reader-text">
		Comment	</label>
	<div id="wp-replycontent-wrap" class="wp-core-ui wp-editor-wrap html-active"><div id="wp-replycontent-editor-container" class="wp-editor-container"><div id="qt_replycontent_toolbar" class="quicktags-toolbar hide-if-no-js"></div><textarea class="wp-editor-area" rows="20" cols="40" name="replycontent" id="replycontent"></textarea></div>
</div>

	</div>

	<div id="edithead" style="display:none;">
		<div class="inside">
		<label for="author-name">Name</label>
		<input type="text" name="newcomment_author" size="50" value="" id="author-name" />
		</div>

		<div class="inside">
		<label for="author-email">Email</label>
		<input type="text" name="newcomment_author_email" size="50" class="code" value="" id="author-email" />
		</div>

		<div class="inside">
		<label for="author-url">URL</label>
		<input type="text" id="author-url" name="newcomment_author_url" class="code" size="103" value="" />
		</div>
	</div>

	<div id="replysubmit" class="submit">
		<p class="reply-submit-buttons">
			<button type="button" class="save button button-primary">
				<span id="addbtn" style="display: none;">Add Comment</span>
				<span id="savebtn" style="display: none;">Update Comment</span>
				<span id="replybtn" style="display: none;">Submit Reply</span>
			</button>
			<button type="button" class="cancel button">Cancel</button>
			<span class="waiting spinner"></span>
		</p>
		<div class="notice notice-error notice-alt inline hidden"><p class="error"></p></div>	</div>

	<input type="hidden" name="action" id="action" value="" />
	<input type="hidden" name="comment_ID" id="comment_ID" value="" />
	<input type="hidden" name="comment_post_ID" id="comment_post_ID" value="" />
	<input type="hidden" name="status" id="status" value="" />
	<input type="hidden" name="position" id="position" value="1" />
	<input type="hidden" name="checkbox" id="checkbox" value="0" />
	<input type="hidden" name="mode" id="mode" value="single" />
	<input type="hidden" id="_ajax_nonce-replyto-comment" name="_ajax_nonce-replyto-comment" value="{{NONCE_REPLY}}" /><input type="hidden" id="_wp_unfiltered_html_comment" name="_wp_unfiltered_html_comment" value="{{NONCE_UNFILTERED}}" />	</fieldset>
	</td></tr></tbody></table>
	</form>
	

