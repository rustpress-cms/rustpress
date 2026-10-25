
		<form method="get">
		<table style="display: none"><tbody id="inlineedit">
		{{LOOP}}			<tr id="{{ID}}" class="{{CLASSES}}" style="display: none">
			<td colspan="{{COLSPAN}}" class="colspanchange">
			<div class="inline-edit-wrapper" role="region" aria-labelledby="{{BQ}}-edit-legend">
			<fieldset class="inline-edit-col-left">
				<legend class="inline-edit-legend" id="{{BQ}}-edit-legend">{{LEGEND}}</legend>
				<div class="inline-edit-col">

				{{if TITLE}}
					{{if BULK}}
						<div id="bulk-title-div">
							<div id="bulk-titles"></div>
						</div>

					{{else}}
						<label>
							<span class="title">Title</span>
							<span class="input-text-wrap"><input type="text" name="post_title" class="ptitle" value="" /></span>
						</label>

						{{if VIEWABLE}}
							<label>
								<span class="title">Slug</span>
								<span class="input-text-wrap"><input type="text" name="post_name" value="" autocomplete="off" spellcheck="false" /></span>
							</label>

						{{end}}
					{{end}}
				{{end}}
				{{if QUICK}}					<fieldset class="inline-edit-date">
						<legend><span class="title">Date</span></legend>
						{{DATE}}					</fieldset>
					<br class="clear" />
				{{end}}
				{{AUTHOR_QUICK}}
				{{if QUICK_PUBLISH}}
					<div class="inline-edit-group wp-clearfix">
						<label class="alignleft">
							<span class="title">Password</span>
							<span class="input-text-wrap"><input type="text" name="post_password" class="inline-edit-password-input ltr" value="" /></span>
						</label>

						<span class="alignleft inline-edit-or">
							&ndash;OR&ndash;						</span>
						<label class="alignleft inline-edit-private">
							<input type="checkbox" name="keep_private" value="private" />
							<span class="checkbox-title">Private</span>
						</label>
					</div>

				{{end}}
				</div>
			</fieldset>

			{{if HIER_TAX}}
				<fieldset class="inline-edit-col-center inline-edit-categories">
					<div class="inline-edit-col">

					{{each HIER}}
						<span class="title inline-edit-categories-label">{{TAX_LABEL}}</span>
						<input type="hidden" name="{{TAX_INPUT}}" value="0" />
						<ul class="cat-checklist {{TAX_NAME}}-checklist">
							{{TAX_CHECKLIST}}						</ul>

					{{end}}
					</div>
				</fieldset>

			{{end}}
			<fieldset class="inline-edit-col-right">
				<div class="inline-edit-col">

				{{AUTHOR_BULK}}
				{{if PAGE_ATTR}}
					{{if HIERARCHICAL}}
						<label>
							<span class="title">Parent</span>
							{{PARENT}}						</label>

					{{end}}
					{{if QUICK}}
						<label>
							<span class="title">Order</span>
							<span class="input-text-wrap"><input type="text" name="menu_order" class="inline-edit-menu-order-input" value="{{MENU_ORDER}}" /></span>
						</label>

					{{end}}
				{{end}}
				{{if HAS_TEMPLATES}}
					<label>
						<span class="title">Template</span>
						<select name="page_template">
							{{if BULK}}							<option value="-1">&mdash; No Change &mdash;</option>
							{{end}}														<option value="default">{{DEFAULT_TEMPLATE}}</option>
							{{TEMPLATES}}						</select>
					</label>

				{{end}}
				{{if FLAT_TAX}}
					{{each FLAT}}
						{{if ITEM_CAN}}														<div class="inline-edit-tags-wrap">
							<label class="inline-edit-tags">
								<span class="title">{{TAX_LABEL}}</span>
								<textarea data-wp-taxonomy="{{TAX_NAME}}" cols="22" rows="1" name="tax_input[{{TAX_NAME}}]" class="tax_input_{{TAX_NAME}}" aria-describedby="inline-edit-{{TAX_NAME}}-desc"></textarea>
							</label>
							<p class="howto" id="inline-edit-{{TAX_NAME}}-desc">{{TAX_COMMAS}}</p>
							</div>
						{{end}}
					{{end}}
				{{end}}
				{{if COMMENTS_OR_PINGS}}
					{{if BULK}}
						<div class="inline-edit-group wp-clearfix">

						{{if COMMENTS}}
							<label class="alignleft">
								<span class="title">Comments</span>
								<select name="comment_status">
									<option value="">&mdash; No Change &mdash;</option>
									<option value="open">Allow</option>
									<option value="closed">Do not allow</option>
								</select>
							</label>

						{{end}}
						{{if TRACKBACKS}}
							<label class="alignright">
								<span class="title">Pings</span>
								<select name="ping_status">
									<option value="">&mdash; No Change &mdash;</option>
									<option value="open">Allow</option>
									<option value="closed">Do not allow</option>
								</select>
							</label>

						{{end}}
						</div>

					{{else}}
						<div class="inline-edit-group wp-clearfix">

						{{if COMMENTS}}
							<label class="alignleft">
								<input type="checkbox" name="comment_status" value="open" />
								<span class="checkbox-title">Allow Comments</span>
							</label>

						{{end}}
						{{if TRACKBACKS}}
							<label class="alignleft">
								<input type="checkbox" name="ping_status" value="open" />
								<span class="checkbox-title">Allow Pings</span>
							</label>

						{{end}}
						</div>

					{{end}}
				{{end}}
					<div class="inline-edit-group wp-clearfix">

						<label class="inline-edit-status alignleft">
							<span class="title">Status</span>
							<select name="_status">
								{{each STATUS}}									<option value="{{VALUE}}">{{TEXT}}</option>
									{{end}}							</select>
						</label>

						{{if STICKY}}
							{{if BULK}}
								<label class="alignright">
									<span class="title">Sticky</span>
									<select name="sticky">
										<option value="-1">&mdash; No Change &mdash;</option>
										<option value="sticky">Sticky</option>
										<option value="unsticky">Not Sticky</option>
									</select>
								</label>

							{{else}}
								<label class="alignleft">
									<input type="checkbox" name="sticky" value="sticky" />
									<span class="checkbox-title">Make this post sticky</span>
								</label>

							{{end}}
						{{end}}
					</div>

				{{if FORMATS}}					
					<label class="alignleft">
						<span class="title">Format</span>
						<select name="post_format">
							<option value="-1">&mdash; No Change &mdash;</option>
							<option value="0">{{STANDARD}}</option>
															{{each FORMAT_LIST}}									<option value="{{VALUE}}">{{TEXT}}</option>
								{{end}}													</select>
					</label>

				{{end}}
				</div>
			</fieldset>

			
			<div class="submit inline-edit-save">
				{{if QUICK}}					{{NONCE_FIELD}}					<button type="button" class="button button-primary save">Update</button>
				{{else}}					{{SUBMIT}}				{{end}}
				<button type="button" class="button cancel">Cancel</button>

				{{if QUICK}}					<span class="spinner"></span>
				{{end}}
				<input type="hidden" name="post_view" value="{{M}}" />
				<input type="hidden" name="screen" value="{{SCREEN_ID}}" />
				{{if QUICK_NOAUTHOR}}					<input type="hidden" name="post_author" value="{{POST_AUTHOR}}" />
				{{end}}
				{{NOTICE}}			</div>
		</div> <!-- end of .inline-edit-wrapper -->

			</td></tr>

			{{ENDLOOP}}		</tbody></table>
		</form>
