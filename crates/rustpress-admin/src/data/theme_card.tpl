<div class="theme">
			<div class="theme-screenshot">
			<img src="{{site}}/wp-content/themes/{{id}}/screenshot.png?ver={{version}}" alt="" />
		</div>
	
		{{update_notice}}<button type="button" class="more-details" id="{{id}}-action"
		aria-label="⟦a¦theme¦View Theme Details for ⟪%s¦{{name}}⟫⟧"
	>⟦r¦¦Theme Details⟧</button>
	<div class="theme-author">
		⟦r¦¦By ⟪%s¦{{author}}⟫⟧	</div>

	<div class="theme-id-container">
					<h2 class="theme-name" id="{{id}}-name">{{name}}</h2>
		
		<div class="theme-actions">
								<a class="button button-compact activate"
				href="{{site}}/wp-admin/themes.php?action=activate&#038;stylesheet={{id}}&#038;_wpnonce={{nonce}}"
				aria-label="⟦a¦theme¦Activate ⟪%s¦{{ data.name }}⟫⟧"
			>⟦r¦¦Activate⟧</a>

							<a class="button button-compact button-primary load-customize hide-if-no-customize"
					href="{{customize}}"
					aria-label="⟦a¦theme¦Live Preview ⟪%s¦{{ data.name }}⟫⟧"
				>⟦r¦¦Live Preview⟧</a>
					
		</div>
	</div>
</div>
