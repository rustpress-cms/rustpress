
	<p class="media-types"><label><input type="radio" name="media_type" value="image" id="image-only"{image_checked} /> {image}</label> &nbsp; &nbsp; <label><input type="radio" name="media_type" value="generic" id="not-image"{generic_checked} /> {generic}</label></p>
	<p class="media-types media-types-required-info">{required_message}</p>
	<table class="describe {table_class}"><tbody>
		<tr>
			<th scope="row" class="label" style="width:130px;">
				<label for="src"><span class="alignleft">{url}</span> {indicator}</label>
				<span class="alignright" id="status_img"></span>
			</th>
			<td class="field"><input id="src" name="src" value="" type="text" required onblur="addExtImage.getImageData()" /></td>
		</tr>

		<tr>
			<th scope="row" class="label">
				<label for="title"><span class="alignleft">{title}</span> {indicator}</label>
			</th>
			<td class="field"><input id="title" name="title" value="" type="text" required /></td>
		</tr>

		<tr class="not-image"><td></td><td><p class="help">{link_text}</p></td></tr>

		<tr class="image-only">
			<th scope="row" class="label">
				<label for="alt"><span class="alignleft">{alt}</span> {indicator}</label>
			</th>
			<td class="field"><input id="alt" name="alt" value="" type="text" required />
			<p class="help">{alt_help}</p></td>
		</tr>
{blank}
		<tr class="image-only">
			<th scope="row" class="label">
				<label for="caption"><span class="alignleft">{caption}</span></label>
			</th>
			<td class="field"><textarea id="caption" name="caption"></textarea></td>
		</tr>
		<tr class="align image-only">
			<th scope="row" class="label"><p><label for="align">{alignment}</label></p></th>
			<td class="field">
				<input name="align" id="align-none" value="none" onclick="addExtImage.align='align'+this.value" type="radio"{checked_none} />
				<label for="align-none" class="align image-align-none-label">{none}</label>
				<input name="align" id="align-left" value="left" onclick="addExtImage.align='align'+this.value" type="radio"{checked_left} />
				<label for="align-left" class="align image-align-left-label">{left}</label>
				<input name="align" id="align-center" value="center" onclick="addExtImage.align='align'+this.value" type="radio"{checked_center} />
				<label for="align-center" class="align image-align-center-label">{center}</label>
				<input name="align" id="align-right" value="right" onclick="addExtImage.align='align'+this.value" type="radio"{checked_right} />
				<label for="align-right" class="align image-align-right-label">{right}</label>
			</td>
		</tr>

		<tr class="image-only">
			<th scope="row" class="label">
				<label for="url"><span class="alignleft">{link_to}</span></label>
			</th>
			<td class="field"><input id="url" name="url" value="" type="text" /><br />

			<button type="button" class="button" value="" onclick="document.forms[0].url.value=null">{none}</button>
			<button type="button" class="button" value="" onclick="document.forms[0].url.value=document.forms[0].src.value">{link_image}</button>
			<p class="help">{link_help}</p></td>
		</tr>
		<tr class="image-only">
			<td></td>
			<td>
				<input type="button" class="button" id="go_button" style="color:#bbb;" onclick="addExtImage.insert()" value="{insert_attr}" />
			</td>
		</tr>
		<tr class="not-image">
			<td></td>
			<td>
				{insert_button}
			</td>
		</tr>
	</tbody></table>
