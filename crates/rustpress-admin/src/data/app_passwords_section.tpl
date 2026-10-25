											<div class="application-passwords hide-if-no-js" id="application-passwords-section">
							<h2>Application Passwords</h2>
							<p>Application passwords allow authentication via non-interactive systems, such as XML-RPC or the REST API, without providing your actual password. Application passwords can be easily revoked. They cannot be used for traditional logins to your website.</p>
															
																	<div class="create-application-password form-wrap">
										<div class="form-field">
											<label for="new_application_password_name">New Application Password Name</label>
											<input type="text" size="30" id="new_application_password_name" name="new_application_password_name" class="input ltr" aria-required="true" aria-describedby="new_application_password_name_desc" spellcheck="false" />
											<p class="description" id="new_application_password_name_desc">Required to create an Application Password, but not to update the user.</p>
										</div>

										
										<button type="button" name="do_new_application_password" id="do_new_application_password" class="button button-secondary">Add Application Password</button>
									</div>
									
								<div class="application-passwords-list-table-wrapper">
											<div class="tablenav top">
						<div class="alignleft actions bulkactions">
							</div>
			<div class="tablenav-pages no-pages"><span class="displaying-num">0 items</span></div>			<br class="clear" />
		</div>
		<table class="wp-list-table widefat fixed striped table-view-list application-passwords-user">
			<thead>
	<tr>
		<th scope="col" id='name' class='manage-column column-name column-primary'  >Name</th><th scope="col" id='created' class='manage-column column-created'  >Created</th><th scope="col" id='last_used' class='manage-column column-last_used'  >Last Used</th><th scope="col" id='last_ip' class='manage-column column-last_ip'  >Last IP</th><th scope="col" id='revoke' class='manage-column column-revoke'  >Revoke</th>	</tr>
	</thead>

	<tbody id="the-list"
				>
		{{ROWS}}	</tbody>

	<tfoot>
	<tr>
		<th scope="col"  class='manage-column column-name column-primary'  >Name</th><th scope="col"  class='manage-column column-created'  >Created</th><th scope="col"  class='manage-column column-last_used'  >Last Used</th><th scope="col"  class='manage-column column-last_ip'  >Last IP</th><th scope="col"  class='manage-column column-revoke'  >Revoke</th>	</tr>
	</tfoot>

</table>
				<div class="tablenav bottom">
							<div class="alignright">
					<button type="button" name="revoke-all-application-passwords" id="revoke-all-application-passwords" class="button delete">Revoke all application passwords</button>
				</div>
						<div class="alignleft actions bulkactions">
							</div>
			<div class="tablenav-pages no-pages"><span class="displaying-num">0 items</span></div>			<br class="clear" />
		</div>
										</div>
													</div>
					
					
					
				
				