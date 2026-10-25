	<div class="imgedit-wrap wp-clearfix">
	<div id="imgedit-panel-{{ID}}">
	{{NOTE}}	<div class="imgedit-panel-content imgedit-panel-tools wp-clearfix">
		<div class="imgedit-menu wp-clearfix">
			<button type="button" onclick="imageEdit.toggleCropTool( {{ID}}, '{{NONCE}}', this );" aria-expanded="false" aria-controls="imgedit-crop" class="imgedit-crop button disabled" disabled>Crop</button>
			<button type="button" class="imgedit-scale button" onclick="imageEdit.toggleControls(this);" aria-expanded="false" aria-controls="imgedit-scale">Scale</button>
			<div class="imgedit-rotate-menu-container">
				<button type="button" aria-controls="imgedit-rotate-menu" class="imgedit-rotate button" aria-expanded="false" onclick="imageEdit.togglePopup(this)" onblur="imageEdit.monitorPopup()">Image Rotation</button>
				<div id="imgedit-rotate-menu" class="imgedit-popup-menu">
								<button type="button" class="imgedit-rleft button" onkeydown="imageEdit.browsePopup(event, this)" onclick="imageEdit.rotate( 90, {{ID}}, '{{NONCE}}', this)" onblur="imageEdit.monitorPopup()">Rotate 90&deg; left</button>
					<button type="button" class="imgedit-rright button" onkeydown="imageEdit.browsePopup(event, this)" onclick="imageEdit.rotate(-90, {{ID}}, '{{NONCE}}', this)" onblur="imageEdit.monitorPopup()">Rotate 90&deg; right</button>
					<button type="button" class="imgedit-rfull button" onkeydown="imageEdit.browsePopup(event, this)" onclick="imageEdit.rotate(180, {{ID}}, '{{NONCE}}', this)" onblur="imageEdit.monitorPopup()">Rotate 180&deg;</button>
									<hr />
					<button type="button" onkeydown="imageEdit.browsePopup(event, this)" onclick="imageEdit.flip(1, {{ID}}, '{{NONCE}}', this)" onblur="imageEdit.monitorPopup()" class="imgedit-flipv button">Flip vertical</button>
					<button type="button" onkeydown="imageEdit.browsePopup(event, this)" onclick="imageEdit.flip(2, {{ID}}, '{{NONCE}}', this)" onblur="imageEdit.monitorPopup()" class="imgedit-fliph button">Flip horizontal</button>
									</div>
			</div>
		</div>
		<div class="imgedit-submit imgedit-menu">
			<button type="button" id="image-undo-{{ID}}" onclick="imageEdit.undo({{ID}}, '{{NONCE}}', this)" class="imgedit-undo button disabled" disabled>Undo</button>
			<button type="button" id="image-redo-{{ID}}" onclick="imageEdit.redo({{ID}}, '{{NONCE}}', this)" class="imgedit-redo button disabled" disabled>Redo</button>
			<button type="button" onclick="imageEdit.close({{ID}}, 1)" class="button imgedit-cancel-btn">Cancel Editing</button>
			<button type="button" onclick="imageEdit.save({{ID}}, '{{NONCE}}')" disabled="disabled" class="button button-primary imgedit-submit-btn">Save Edits</button>
		</div>
	</div>

	<div class="imgedit-panel-content wp-clearfix">
		<div class="imgedit-tools">
			<input type="hidden" id="imgedit-nonce-{{ID}}" value="{{NONCE}}" />
			<input type="hidden" id="imgedit-sizer-{{ID}}" value="{{SIZER}}" />
			<input type="hidden" id="imgedit-history-{{ID}}" value="" />
			<input type="hidden" id="imgedit-undone-{{ID}}" value="0" />
			<input type="hidden" id="imgedit-selection-{{ID}}" value="" />
			<input type="hidden" id="imgedit-x-{{ID}}" value="{{WIDTH}}" />
			<input type="hidden" id="imgedit-y-{{ID}}" value="{{HEIGHT}}" />

			<div id="imgedit-crop-{{ID}}" class="imgedit-crop-wrap">
			<div class="imgedit-crop-grid"></div>
			<img id="image-preview-{{ID}}" onload="imageEdit.imgLoaded('{{ID}}')"
				src="/wp-admin/admin-ajax.php?action=imgedit-preview&amp;_ajax_nonce={{NONCE}}&amp;postid={{ID}}&amp;rand={{RAND}}" alt="" />
			</div>
		</div>
		<div class="imgedit-settings">
			<div class="imgedit-tool-active">
				<div class="imgedit-group">
				<div id="imgedit-scale" tabindex="-1" class="imgedit-group-controls">
					<div class="imgedit-group-top">
						<h2>Scale Image</h2>
						<button type="button" class="dashicons dashicons-editor-help imgedit-help-toggle" onclick="imageEdit.toggleHelp(this);" aria-expanded="false"><span class="screen-reader-text">
						Scale Image Help						</span></button>
						<div class="imgedit-help">
						<p>You can proportionally scale the original image. For best results, scaling should be done before you crop, flip, or rotate. Images can only be scaled down, not up.</p>
						</div>
												<p>
							Original dimensions <span class="imgedit-original-dimensions">{{WIDTH}} &times; {{HEIGHT}}</span>						</p>
												<div class="imgedit-submit">
						<fieldset class="imgedit-scale-controls">
							<legend>New dimensions:</legend>
							<div class="nowrap">
							<label for="imgedit-scale-width-{{ID}}" class="screen-reader-text">
							Width							</label>
							<input type="number" step="1" min="0" max="{{WIDTH}}" aria-describedby="imgedit-scale-warn-{{ID}}"  id="imgedit-scale-width-{{ID}}" onkeyup="imageEdit.scaleChanged({{ID}}, 1, this)" onblur="imageEdit.scaleChanged({{ID}}, 1, this)" value="{{WIDTH}}" />
							<span class="imgedit-separator" aria-hidden="true">&times;</span>
							<label for="imgedit-scale-height-{{ID}}" class="screen-reader-text">
							Height							</label>
							<input type="number" step="1" min="0" max="{{HEIGHT}}" aria-describedby="imgedit-scale-warn-{{ID}}" id="imgedit-scale-height-{{ID}}" onkeyup="imageEdit.scaleChanged({{ID}}, 0, this)" onblur="imageEdit.scaleChanged({{ID}}, 0, this)" value="{{HEIGHT}}" />
							<button id="imgedit-scale-button" type="button" onclick="imageEdit.action({{ID}}, '{{NONCE}}', 'scale')" class="button button-primary">Scale</button>
							</div>
							<span class="imgedit-scale-warn" id="imgedit-scale-warn-{{ID}}"><span class="dashicons dashicons-warning" aria-hidden="true"></span>Images cannot be scaled to a size larger than the original.</span>
						</fieldset>
						</div>
					</div>
				</div>
			</div>

{{RESTORE}}					<div class="imgedit-group">
				<div id="imgedit-crop" tabindex="-1" class="imgedit-group-controls">
				<div class="imgedit-group-top">
					<h2>Crop Image</h2>
					<button type="button" class="dashicons dashicons-editor-help imgedit-help-toggle" onclick="imageEdit.toggleHelp(this);" aria-expanded="false"><span class="screen-reader-text">
					Image Crop Help					</span></button>
					<div class="imgedit-help">
						<p>To crop the image, click on it and drag to make your selection.</p>
						<p><strong>Crop Aspect Ratio</strong><br />
						The aspect ratio is the relationship between the width and height. You can preserve the aspect ratio by holding down the shift key while resizing your selection. Use the input box to specify the aspect ratio, e.g. 1:1 (square), 4:3, 16:9, etc.</p>

						<p><strong>Crop Selection</strong><br />
						Once you have made your selection, you can adjust it by entering the size in pixels. The minimum selection size is the thumbnail size as set in the Media settings.</p>
					</div>
				</div>
				<fieldset class="imgedit-crop-ratio">
					<legend>Aspect ratio:</legend>
					<div class="nowrap">
					<label for="imgedit-crop-width-{{ID}}" class="screen-reader-text">
					crop ratio width					</label>
					<input type="number" step="1" min="1" id="imgedit-crop-width-{{ID}}" onkeyup="imageEdit.setRatioSelection({{ID}}, 0, this)" onblur="imageEdit.setRatioSelection({{ID}}, 0, this)" />
					<span class="imgedit-separator" aria-hidden="true">:</span>
					<label for="imgedit-crop-height-{{ID}}" class="screen-reader-text">
					crop ratio height					</label>
					<input  type="number" step="1" min="0" id="imgedit-crop-height-{{ID}}" onkeyup="imageEdit.setRatioSelection({{ID}}, 1, this)" onblur="imageEdit.setRatioSelection({{ID}}, 1, this)" />
					</div>
				</fieldset>
				<fieldset id="imgedit-crop-sel-{{ID}}" class="imgedit-crop-sel">
					<legend>Selection:</legend>
					<div class="nowrap">
					<label for="imgedit-sel-width-{{ID}}" class="screen-reader-text">
					selection width					</label>
					<input  type="number" step="1" min="0" id="imgedit-sel-width-{{ID}}" onkeyup="imageEdit.setNumSelection({{ID}}, this)" onblur="imageEdit.setNumSelection({{ID}}, this)" />
					<span class="imgedit-separator" aria-hidden="true">&times;</span>
					<label for="imgedit-sel-height-{{ID}}" class="screen-reader-text">
					selection height					</label>
					<input  type="number" step="1" min="0" id="imgedit-sel-height-{{ID}}" onkeyup="imageEdit.setNumSelection({{ID}}, this)" onblur="imageEdit.setNumSelection({{ID}}, this)" />
					</div>
				</fieldset>
				<fieldset id="imgedit-crop-sel-{{ID}}" class="imgedit-crop-sel">
					<legend>Starting Coordinates:</legend>
					<div class="nowrap">
					<label for="imgedit-start-x-{{ID}}" class="screen-reader-text">
					horizontal start position					</label>
					<input  type="number" step="1" min="0" id="imgedit-start-x-{{ID}}" onkeyup="imageEdit.setNumSelection({{ID}}, this)" onblur="imageEdit.setNumSelection({{ID}}, this)" value="0" />
					<span class="imgedit-separator" aria-hidden="true">&times;</span>
					<label for="imgedit-start-y-{{ID}}" class="screen-reader-text">
					vertical start position					</label>
					<input  type="number" step="1" min="0" id="imgedit-start-y-{{ID}}" onkeyup="imageEdit.setNumSelection({{ID}}, this)" onblur="imageEdit.setNumSelection({{ID}}, this)" value="0" />
					</div>
				</fieldset>
				<div class="imgedit-crop-apply imgedit-menu container">
					<button class="button button-primary" type="button" onclick="imageEdit.handleCropToolClick( {{ID}}, '{{NONCE}}', this );" class="imgedit-crop-apply button">Apply Crop</button> <button type="button" onclick="imageEdit.handleCropToolClick( {{ID}}, '{{NONCE}}', this );" class="imgedit-crop-clear button" disabled="disabled">Clear Crop</button>
				</div>
			</div>
		</div>
	</div>

			</div>
	</div>

	</div>

	<div class="imgedit-wait" id="imgedit-wait-{{ID}}"></div>
	<div class="hidden" id="imgedit-leaving-{{ID}}">There are unsaved changes that will be lost. 'OK' to continue, 'Cancel' to return to the Image Editor.</div>
	</div>
	