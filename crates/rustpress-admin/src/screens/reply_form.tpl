
<div id="ajax-response"></div>

<form method="get">
	<table style="display:none;"><tbody id="com-reply"><tr id="replyrow" class="inline-edit-row" style="display:none;"><td colspan="5" class="colspanchange">
	<fieldset class="comment-reply">
	<legend>
		<span class="hidden" id="editlegend">@@Edit Comment@@</span>
		<span class="hidden" id="replyhead">@@Reply to Comment@@</span>
		<span class="hidden" id="addhead">@@Add Comment@@</span>
	</legend>

	<div id="replycontainer">
	<label for="replycontent" class="screen-reader-text">
		@@Comment@@	</label>
	<div id="wp-replycontent-wrap" class="wp-core-ui wp-editor-wrap html-active"><link rel='stylesheet' id='editor-buttons-css' href='{{editor_css}}' media='all' />
<div id="wp-replycontent-editor-container" class="wp-editor-container"><div id="qt_replycontent_toolbar" class="quicktags-toolbar hide-if-no-js"></div><textarea class="wp-editor-area" rows="20" cols="40" name="replycontent" id="replycontent"></textarea></div>
</div>

	</div>

	<div id="edithead" style="display:none;">
		<div class="inside">
		<label for="author-name">@@Name@@</label>
		<input type="text" name="newcomment_author" size="50" value="" id="author-name" />
		</div>

		<div class="inside">
		<label for="author-email">@@Email@@</label>
		<input type="text" name="newcomment_author_email" size="50" class="code" value="" id="author-email" />
		</div>

		<div class="inside">
		<label for="author-url">@@URL@@</label>
		<input type="text" id="author-url" name="newcomment_author_url" class="code" size="103" value="" />
		</div>
	</div>

	<div id="replysubmit" class="submit">
		<p class="reply-submit-buttons">
			<button type="button" class="save button button-primary">
				<span id="addbtn" style="display: none;">@@Add Comment@@</span>
				<span id="savebtn" style="display: none;">@@Update Comment@@</span>
				<span id="replybtn" style="display: none;">@@Submit Reply@@</span>
			</button>
			<button type="button" class="cancel button">@@Cancel@@</button>
			<span class="waiting spinner"></span>
		</p>
		<div class="notice notice-error notice-alt inline hidden"><p class="error"></p></div>	</div>

	<input type="hidden" name="action" id="action" value="" />
	<input type="hidden" name="comment_ID" id="comment_ID" value="" />
	<input type="hidden" name="comment_post_ID" id="comment_post_ID" value="" />
	<input type="hidden" name="status" id="status" value="" />
	<input type="hidden" name="position" id="position" value="-1" />
	<input type="hidden" name="checkbox" id="checkbox" value="1" />
	<input type="hidden" name="mode" id="mode" value="detail" />
	<input type="hidden" id="_ajax_nonce-replyto-comment" name="_ajax_nonce-replyto-comment" value="{{reply_nonce}}" /><input type="hidden" id="_wp_unfiltered_html_comment" name="_wp_unfiltered_html_comment" value="{{unfiltered}}" />	</fieldset>
	</td></tr></tbody></table>
	</form>
	<div class="hidden" id="trash-undo-holder">
	<div class="trash-undo-inside">
		@@Comment by %s moved to the Trash.@@		<span class="undo untrash"><a href="#">@@Undo@@</a></span>
	</div>
</div>
<div class="hidden" id="spam-undo-holder">
	<div class="spam-undo-inside">
		@@Comment by %s marked as spam.@@		<span class="undo unspam"><a href="#">@@Undo@@</a></span>
	</div>
</div>

