	<script id="tmpl-community-events-attend-event-near" type="text/template">
		@@Attend an upcoming event near %s.@@	</script>

	<script id="tmpl-community-events-could-not-locate" type="text/template">
		@@%s could not be located. Please try another nearby city. For example: Kansas City; Springfield; Portland.@@	</script>

	<script id="tmpl-community-events-event-list" type="text/template">
		<# _.each( data.events, function( event ) { #>
			<li class="event event-{{ event.type }} wp-clearfix">
				<div class="event-info">
					<div class="dashicons event-icon" aria-hidden="true"></div>
					<div class="event-info-inner">
						<a class="event-title" href="{{ event.url }}">{{ event.title }}</a>
						<# if ( event.type ) {
							const titleCaseEventType = event.type.replace(
								/\w\S*/g,
								function ( type ) { return type.charAt(0).toUpperCase() + type.substr(1).toLowerCase(); }
							);
						#>
							{{ 'wordcamp' === event.type ? 'WordCamp' : titleCaseEventType }}
							<span class="ce-separator" aria-hidden="true"></span>
						<# } #>
						<span class="event-city">{{ event.location.location }}</span>
					</div>
				</div>

				<div class="event-date-time">
					<span class="event-date">{{ event.user_formatted_date }}</span>
					<# if ( 'meetup' === event.type ) { #>
						<span class="event-time">
							{{ event.user_formatted_time }} {{ event.timeZoneAbbreviation }}
						</span>
					<# } #>
				</div>
			</li>
		<# } ) #>

		<# if ( data.events.length <= 2 ) { #>
			<li class="event-none">
				@@Want more events? <a href="%s">Help organize the next one</a>!@@			</li>
		<# } #>

	</script>

	<script id="tmpl-community-events-no-upcoming-events" type="text/template">
		<li class="event-none">
			<# if ( data.location.description ) { #>
				@@There are no events scheduled near %1$s at the moment. Would you like to <a href="%2$s">organize a WordPress event</a>?@@
			<# } else { #>
				@@There are no events scheduled near you at the moment. Would you like to <a href="%s">organize a WordPress event</a>?@@			<# } #>
		</li>
	</script>
	
