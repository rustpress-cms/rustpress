				<tr>
			<th scope="row">
				<label for="page_for_privacy_policy">
					{{change_label}}				</label>
			</th>
			<td>
				<form method="post">
					<input type="hidden" name="action" value="set-privacy-page" />
					{{dropdown}}
<input type="hidden" id="_wpnonce" name="_wpnonce" value="{{nonce:set-privacy-page}}" /><input type="hidden" name="_wp_http_referer" value="{{referer}}" /><input type="submit" name="submit" id="set-page" class="button button-primary" value="Use This Page"  />				</form>
			</td>
		</tr>
