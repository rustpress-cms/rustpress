( ( [ dropdownId, homeUrl ] ) => {
		const dropdown = document.getElementById( dropdownId );
		function onSelectChange() {
			setTimeout( () => {
				if ( 'escape' === dropdown.dataset.lastkey ) {
					return;
				}
				// Only navigate if a valid term is selected (not the default "Select [taxonomy]" option)
				if ( dropdown.value && dropdown.value !== '-1' && dropdown instanceof HTMLSelectElement ) {
					const url = new URL( homeUrl );
					url.searchParams.set( dropdown.name, dropdown.value );
					location.href = url.href;
				}
			}, 250 );
		}
		function onKeyUp( event ) {
			if ( 'Escape' === event.key ) {
				dropdown.dataset.lastkey = 'escape';
			} else {
				delete dropdown.dataset.lastkey;
			}
		}
		function onClick() {
			delete dropdown.dataset.lastkey;
		}
		dropdown.addEventListener( 'keyup', onKeyUp );
		dropdown.addEventListener( 'click', onClick );
		dropdown.addEventListener( 'change', onSelectChange );
	} )( __EXPORTS__ );