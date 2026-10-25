						<div class="imgedit-group">
				<div class="imgedit-group-top">
					<h2><button type="button" onclick="imageEdit.toggleHelp(this);" class="button-link" aria-expanded="false">Restore original image <span class="dashicons dashicons-arrow-down imgedit-help-toggle"></span></button></h2>
					<div class="imgedit-help imgedit-restore">
					<p>
					Discard any changes and restore the original image. Previously edited copies of the image will not be deleted.					</p>
					<div class="imgedit-submit">
						<input type="button" onclick="imageEdit.action({{ID}}, '{{NONCE}}', 'restore')" class="button button-primary" value="Restore image" 1 />
					</div>
				</div>
			</div>
			</div>
