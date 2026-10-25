	<script type="text/html" id="tmpl-new-application-password">
		<div class="notice notice-success is-dismissible new-application-password-notice" role="alert">
			<p class="application-password-display">
				<label for="new-application-password-value">
					Your new password for <strong>{{ data.name }}</strong> is:				</label>
				<input id="new-application-password-value" type="text" class="code" readonly="readonly" value="{{ data.password }}" />
				<button type="button" class="button copy-button" data-clipboard-text="{{ data.password }}">Copy</button>
				<span class="success hidden" aria-hidden="true">Copied!</span>
			</p>
			<p>Be sure to save this in a safe location. You will not be able to retrieve it.</p>
			<button type="button" class="notice-dismiss">
				<span class="screen-reader-text">
					Dismiss this notice.				</span>
			</button>
		</div>
	</script>

	<script type="text/html" id="tmpl-application-password-row">
		<tr data-uuid="{{ data.uuid }}"><td class="name column-name has-row-actions column-primary" data-colname="Name">{{ data.name }}<button type="button" class="toggle-row"><span class="screen-reader-text">Show more details</span></button></td><td class="created column-created" data-colname="Created"><# print( wp.date.dateI18n( "F j, Y", data.created ) ) #></td><td class="last_used column-last_used" data-colname="Last Used"><# print( data.last_used !== null ? wp.date.dateI18n( "F j, Y", data.last_used ) : '—' ) #></td><td class="last_ip column-last_ip" data-colname="Last IP">{{ data.last_ip || '—' }}</td><td class="revoke column-revoke" data-colname="Revoke"><button type="button" class="button delete" aria-label="Revoke &quot;{{ data.name }}&quot;">Revoke</button></td></tr>	</script>

