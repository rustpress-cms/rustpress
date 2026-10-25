<form method="get">
		<table style="display: none"><tbody id="inlineedit">
					<tr id="inline-edit" class="inline-edit-row inline-edit-row-page quick-edit-row quick-edit-row-page inline-edit-page" style="display: none">
			<td colspan="{{COLSPAN}}" class="colspanchange">
			<div class="inline-edit-wrapper" role="region" aria-labelledby="quick-edit-legend">
			<fieldset class="inline-edit-col-left">
				<legend class="inline-edit-legend" id="quick-edit-legend">Quick Edit</legend>
				<div class="inline-edit-col">

				
					
						<label>
							<span class="title">Title</span>
							<span class="input-text-wrap"><input type="text" name="post_title" class="ptitle" value="" /></span>
						</label>

						
							<label>
								<span class="title">Slug</span>
								<span class="input-text-wrap"><input type="text" name="post_name" value="" autocomplete="off" spellcheck="false" /></span>
							</label>

						
					
				
									<fieldset class="inline-edit-date">
						<legend><span class="title">Date</span></legend>
						{{DATE}}					</fieldset>
					<br class="clear" />
				
				<label class="inline-edit-author"><span class="title">Author</span>{{AUTHORS}}</label>
				
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

				
				</div>
			</fieldset>

			
			<fieldset class="inline-edit-col-right">
				<div class="inline-edit-col">

				
				
					
						<label>
							<span class="title">Parent</span>
							{{PARENT}}
						</label>

					
					
						<label>
							<span class="title">Order</span>
							<span class="input-text-wrap"><input type="text" name="menu_order" class="inline-edit-menu-order-input" value="{{MENU_ORDER}}" /></span>
						</label>

					
				
				
					<label>
						<span class="title">Template</span>
						<select name="page_template">
																					<option value="default">Default template</option>
{{TEMPLATES}}						</select>
					</label>

				
				
				
					
						<div class="inline-edit-group wp-clearfix">

						
							<label class="alignleft">
								<input type="checkbox" name="comment_status" value="open" />
								<span class="checkbox-title">Allow Comments</span>
							</label>

						
						
						</div>

					
				
					<div class="inline-edit-group wp-clearfix">

						<label class="inline-edit-status alignleft">
							<span class="title">Status</span>
							<select name="_status">
																	<option value="publish">Published</option>
																		<option value="future">Scheduled</option>
																		<option value="pending">Pending Review</option>
																		<option value="draft">Draft</option>
																</select>
						</label>

						
					</div>

				
				</div>
			</fieldset>

			
			<div class="submit inline-edit-save">
									<input type="hidden" id="_inline_edit" name="_inline_edit" value="{{NONCE}}" />					<button type="button" class="button button-primary save">Update</button>
				
				<button type="button" class="button cancel">Cancel</button>

									<span class="spinner"></span>
				
				<input type="hidden" name="post_view" value="list" />
				<input type="hidden" name="screen" value="edit-page" />
				
				<div class="notice notice-error notice-alt inline hidden"><p class="error"></p></div>			</div>
		</div> <!-- end of .inline-edit-wrapper -->

			</td></tr>

						<tr id="bulk-edit" class="inline-edit-row inline-edit-row-page bulk-edit-row bulk-edit-row-page bulk-edit-page" style="display: none">
			<td colspan="{{COLSPAN}}" class="colspanchange">
			<div class="inline-edit-wrapper" role="region" aria-labelledby="bulk-edit-legend">
			<fieldset class="inline-edit-col-left">
				<legend class="inline-edit-legend" id="bulk-edit-legend">Bulk Edit</legend>
				<div class="inline-edit-col">

				
					
						<div id="bulk-title-div">
							<div id="bulk-titles"></div>
						</div>

					
				
				
				
				
				</div>
			</fieldset>

			
			<fieldset class="inline-edit-col-right">
				<div class="inline-edit-col">

				<label class="inline-edit-author"><span class="title">Author</span>{{AUTHORS_BULK}}</label>
				
					
						<label>
							<span class="title">Parent</span>
							{{PARENT_BULK}}
						</label>

					
					
				
				
					<label>
						<span class="title">Template</span>
						<select name="page_template">
														<option value="-1">&mdash; No Change &mdash;</option>
																					<option value="default">Default template</option>
{{TEMPLATES}}						</select>
					</label>

				
				
				
					
						<div class="inline-edit-group wp-clearfix">

						
							<label class="alignleft">
								<span class="title">Comments</span>
								<select name="comment_status">
									<option value="">&mdash; No Change &mdash;</option>
									<option value="open">Allow</option>
									<option value="closed">Do not allow</option>
								</select>
							</label>

						
						
						</div>

					
				
					<div class="inline-edit-group wp-clearfix">

						<label class="inline-edit-status alignleft">
							<span class="title">Status</span>
							<select name="_status">
																	<option value="-1">&mdash; No Change &mdash;</option>
																		<option value="publish">Published</option>
																		<option value="future">Scheduled</option>
																		<option value="private">Private</option>
																		<option value="pending">Pending Review</option>
																		<option value="draft">Draft</option>
																</select>
						</label>

						
					</div>

				
				</div>
			</fieldset>

			
			<div class="submit inline-edit-save">
									<input type="submit" name="bulk_edit" id="bulk_edit" class="button button-primary" value="Update"  />				
				<button type="button" class="button cancel">Cancel</button>

				
				<input type="hidden" name="post_view" value="list" />
				<input type="hidden" name="screen" value="edit-page" />
				
				<div class="notice notice-error notice-alt inline hidden"><p class="error"></p></div>			</div>
		</div> <!-- end of .inline-edit-wrapper -->

			</td></tr>

					</tbody></table>
		</form>