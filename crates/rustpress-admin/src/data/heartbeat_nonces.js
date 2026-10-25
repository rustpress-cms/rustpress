( function( $ ) {
		var check, timeout;

		function schedule() {
			check = false;
			window.clearTimeout( timeout );
			timeout = window.setTimeout( function() { check = true; }, 300000 );
		}

		$( document ).on( 'heartbeat-send.wp-refresh-nonces', function( e, data ) {
			var post_id, $authCheck = $( '#wp-auth-check-wrap' );

			if ( check || ( $authCheck.length && ! $authCheck.hasClass( 'hidden' ) ) ) {
				if ( ( post_id = $( '#post_ID' ).val() ) && $( '#_wpnonce' ).val() ) {
					data['wp-refresh-metabox-loader-nonces'] = {
						post_id: post_id
					};
				}
			}
		}).on( 'heartbeat-tick.wp-refresh-nonces', function( e, data ) {
			var nonces = data['wp-refresh-metabox-loader-nonces'];

			if ( nonces ) {
				if ( nonces.replace ) {
					if ( nonces.replace.metabox_loader_nonce && window._wpMetaBoxUrl && wp.url ) {
						window._wpMetaBoxUrl= wp.url.addQueryArgs( window._wpMetaBoxUrl, { 'meta-box-loader-nonce': nonces.replace.metabox_loader_nonce } );
					}

					if ( nonces.replace._wpnonce ) {
						$( '#_wpnonce' ).val( nonces.replace._wpnonce );
					}
				}
			}
		}).ready( function() {
			schedule();
		});
	} )( jQuery );