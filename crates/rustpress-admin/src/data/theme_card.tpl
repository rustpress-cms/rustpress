<div class="theme">
			<div class="theme-screenshot">
			<img src="{{site}}/wp-content/themes/{{id}}/screenshot.png?ver={{version}}" alt="" />
		</div>
	
		{{update_notice}}<button type="button" class="more-details" id="{{id}}-action"
		aria-label="View Theme Details for {{name}}"
	>Theme Details</button>
	<div class="theme-author">
		By {{author}}	</div>

	<div class="theme-id-container">
					<h2 class="theme-name" id="{{id}}-name">{{name}}</h2>
		
		<div class="theme-actions">
								<a class="button button-compact activate"
				href="{{site}}/wp-admin/themes.php?action=activate&#038;stylesheet={{id}}&#038;_wpnonce={{nonce}}"
				aria-label="Activate {{ data.name }}"
			>Activate</a>

							<a class="button button-compact button-primary load-customize hide-if-no-customize"
					href="{{site}}/wp-admin/site-editor.php?wp_theme_preview={{id}}&#038;return={{return}}"
					aria-label="Live Preview {{ data.name }}"
				>Live Preview</a>
					
		</div>
	</div>
</div>
