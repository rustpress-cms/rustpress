<div class="notice notice-error community-events-errors inline hide-if-js"><p class="hide-if-js">@@This widget requires JavaScript.@@</p><p class="community-events-error-occurred" aria-hidden="true">@@An error occurred. Please try again.@@</p><p class="community-events-could-not-locate" aria-hidden="true"></p></div>	<div id="community-events" class="community-events" aria-hidden="true">
		<div class="activity-block">
			<p>
				<span id="community-events-location-message"></span>

				<button class="button-link community-events-toggle-location" aria-expanded="false">
					<span class="dashicons dashicons-location" aria-hidden="true"></span>
					<span class="community-events-location-edit">@@Select location@@</span>
				</button>
			</p>

			<form class="community-events-form" aria-hidden="true" action="{{admin}}admin-ajax.php" method="post">
				<label for="community-events-location">
					@@City:@@				</label>
								<input id="community-events-location" class="regular-text" type="text" name="community-events-location" placeholder="@@Cincinnati@@" />

				<input type="submit" name="community-events-submit" id="community-events-submit" class="button" value="@@Submit@@"  />
				<button class="community-events-cancel button-link" type="button" aria-expanded="false">
					@@Cancel@@				</button>

				<span class="spinner"></span>
			</form>
		</div>

		<ul class="community-events-results activity-block last"></ul>
	</div>

	
	<div class="wordpress-news hide-if-no-js">
		<p class="widget-loading hide-if-no-js">@@Loading&hellip;@@</p><div class="notice notice-error inline hide-if-js"><p>@@This widget requires JavaScript.@@</p></div>	</div>

	<p class="community-events-footer">
		<a href="https://make.wordpress.org/community/meetups-landing-page" target="_blank">@@Meetups@@ <span class="screen-reader-text"> @@(opens in a new tab)@@</span><span aria-hidden="true" class="dashicons dashicons-external"></span></a>
		|

		<a href="https://central.wordcamp.org/schedule/" target="_blank">@@WordCamps@@ <span class="screen-reader-text"> @@(opens in a new tab)@@</span><span aria-hidden="true" class="dashicons dashicons-external"></span></a>
		|

		<a href="https://wordpress.org/news/" target="_blank">@@News@@ <span class="screen-reader-text"> @@(opens in a new tab)@@</span><span aria-hidden="true" class="dashicons dashicons-external"></span></a>	</p>

	