	<script>
	jQuery(function($){
		var preloaded = $(".media-item.preloaded");
		if ( preloaded.length > 0 ) {
			preloaded.each(function(){prepareMediaItem({id:this.id.replace(/[^0-9]/g, '')},'');});
			updateMediaForm();
		}
	});
	</script>
	<div id="sort-buttons" class="hide-if-no-js">
	<span>
		{all_tabs}	<a href="#" id="showall">{show}</a>
	<a href="#" id="hideall" style="display:none;">{hide}</a>
	</span>
		{sort_order}	<a href="#" id="asc">{ascending}</a> |
	<a href="#" id="desc">{descending}</a> |
	<a href="#" id="clear">{clear}</a>
	</div>
	<form enctype="multipart/form-data" method="post" action="{action}" class="{form_class}" id="gallery-form">
		{nonce_field}	<table class="widefat">
	<thead><tr>
	<th>{media}</th>
	<th class="order-head">{order}</th>
	<th class="actions-head">{actions}</th>
	</tr></thead>
	</table>
	<div id="media-items">
				{items}	</div>

	<p class="ml-submit">
		{save_button}	<input type="hidden" name="post_id" id="post_id" value="{post_id}" />
	<input type="hidden" name="type" value="{type}" />
	<input type="hidden" name="tab" value="{tab}" />
	</p>

	<div id="gallery-settings" style="display:none;">
	<div class="title">{gallery_settings}</div>
	<table id="basic" class="describe"><tbody>
		<tr>
		<th scope="row" class="label">
			<label>
			<span class="alignleft">{link_thumbnails}</span>
			</label>
		</th>
		<td class="field">
			<input type="radio" name="linkto" id="linkto-file" value="file" />
			<label for="linkto-file" class="radio">{image_file}</label>

			<input type="radio" checked="checked" name="linkto" id="linkto-post" value="post" />
			<label for="linkto-post" class="radio">{attachment_page}</label>
		</td>
		</tr>

		<tr>
		<th scope="row" class="label">
			<label>
			<span class="alignleft">{order_images}</span>
			</label>
		</th>
		<td class="field">
			<select id="orderby" name="orderby">
				<option value="menu_order" selected="selected">{menu_order}</option>
				<option value="title">{title}</option>
				<option value="post_date">{date_time}</option>
				<option value="rand">{random}</option>
			</select>
		</td>
		</tr>

		<tr>
		<th scope="row" class="label">
			<label>
			<span class="alignleft">{order_colon}</span>
			</label>
		</th>
		<td class="field">
			<input type="radio" checked="checked" name="order" id="order-asc" value="asc" />
			<label for="order-asc" class="radio">{ascending}</label>

			<input type="radio" name="order" id="order-desc" value="desc" />
			<label for="order-desc" class="radio">{descending}</label>
		</td>
		</tr>

		<tr>
		<th scope="row" class="label">
			<label>
			<span class="alignleft">{columns}</span>
			</label>
		</th>
		<td class="field">
			<select id="columns" name="columns">
				<option value="1">1</option>
				<option value="2">2</option>
				<option value="3" selected="selected">3</option>
				<option value="4">4</option>
				<option value="5">5</option>
				<option value="6">6</option>
				<option value="7">7</option>
				<option value="8">8</option>
				<option value="9">9</option>
			</select>
		</td>
		</tr>
	</tbody></table>

	<p class="ml-submit">
	<input type="button" class="button" style="display:none;" onMouseDown="wpgallery.update();" name="insert-gallery" id="insert-gallery" value="{insert_gallery}" />
	<input type="button" class="button" style="display:none;" onMouseDown="wpgallery.update();" name="update-gallery" id="update-gallery" value="{update_gallery}" />
	</p>
	</div>
	</form>
