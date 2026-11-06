		<li id="menu-item-{id}" class="{classes}">
			<div class="menu-item-bar">
				<div class="menu-item-handle">
					<label class="item-title" for="menu-item-checkbox-{id}">
						<input id="menu-item-checkbox-{id}" type="checkbox" class="menu-item-checkbox" data-menu-item-id="{id}" disabled="disabled" />
						<span class="menu-item-title">{title}</span>
						<span class="is-submenu" {submenu_style}>{sub_item}</span>
					</label>
					<span class="item-controls">
						<span class="item-type">{type_label}</span>
						<span class="item-order hide-if-js">
							<a href="{move_up_url}" class="item-move-up" aria-label="{move_up}">&#8593;</a>							|
							<a href="{move_down_url}" class="item-move-down" aria-label="{move_down}">&#8595;</a>						</span>
						<a class="item-edit" id="edit-{id}" href="{edit_url}" aria-label="{edit_item}"><span class="screen-reader-text">{edit}</span></a>					</span>
				</div>
			</div>

			<div class="menu-item-settings wp-clearfix" id="menu-item-settings-{id}">
				{url_field}				<p class="description description-wide">
					<label for="edit-menu-item-title-{id}">
						{navigation_label}<br />
						<input type="text" id="edit-menu-item-title-{id}" class="widefat edit-menu-item-title" name="menu-item-title[{id}]" value="{title_value}" />
					</label>
				</p>
				<p class="field-title-attribute field-attr-title description description-wide">
					<label for="edit-menu-item-attr-title-{id}">
						{title_attribute}<br />
						<input type="text" id="edit-menu-item-attr-title-{id}" class="widefat edit-menu-item-attr-title" name="menu-item-attr-title[{id}]" value="{attr_title}" />
					</label>
				</p>
				<p class="field-link-target description">
					<label for="edit-menu-item-target-{id}">
						<input type="checkbox" id="edit-menu-item-target-{id}" value="_blank" name="menu-item-target[{id}]"{target_checked} />
						{new_tab}					</label>
				</p>
				<div class="description-group">
					<p class="field-css-classes description description-thin">
						<label for="edit-menu-item-classes-{id}">
							{css_classes}<br />
							<input type="text" id="edit-menu-item-classes-{id}" class="widefat code edit-menu-item-classes" name="menu-item-classes[{id}]" value="{classes_value}" />
						</label>
					</p>
					<p class="field-xfn description description-thin">
						<label for="edit-menu-item-xfn-{id}">
							{xfn_label}<br />
							<input type="text" id="edit-menu-item-xfn-{id}" class="widefat code edit-menu-item-xfn" name="menu-item-xfn[{id}]" value="{xfn}" />
						</label>
					</p>
				</div>
				<p class="field-description description description-wide">
					<label for="edit-menu-item-description-{id}">
						{description_label}<br />
						<textarea id="edit-menu-item-description-{id}" class="widefat edit-menu-item-description" rows="3" cols="20" name="menu-item-description[{id}]">{description}</textarea>
						<span class="description">{description_help}</span>
					</label>
				</p>

{t5}
				<div class="field-move-combo description-group">
					<p class="description description-wide">
						<label for="edit-menu-item-parent-{id}">
							{menu_parent}						</label>
						<select class="edit-menu-item-parent widefat" id="edit-menu-item-parent-{id}" name="menu-item-parent[{id}]">
						</select>
					</p>
					<p class="description description-wide">
						<label for="edit-menu-item-order-{id}">
							{menu_order}						</label>
						<select class="edit-menu-item-order widefat" id="edit-menu-item-order-{id}" name="menu-item-order[{id}]">
						</select>
					</p>
				</div>

{t4}
				<fieldset class="field-move hide-if-no-js description description-wide">
					<span class="field-move-visual-label" aria-hidden="true">{move}</span>
					<button type="button" class="button-link menus-move menus-move-up" data-dir="up">{up_one}</button>
					<button type="button" class="button-link menus-move menus-move-down" data-dir="down">{down_one}</button>
					<button type="button" class="button-link menus-move menus-move-left" data-dir="left"></button>
					<button type="button" class="button-link menus-move menus-move-right" data-dir="right"></button>
					<button type="button" class="button-link menus-move menus-move-top" data-dir="top">{to_top}</button>
				</fieldset>

				<div class="menu-item-actions description-wide submitbox">
					{original}
					<a class="item-delete submitdelete deletion" id="delete-{id}" href="{delete_url}">{remove}</a>					<span class="meta-sep hide-if-no-js"> | </span>
					<a class="item-cancel submitcancel hide-if-no-js" id="cancel-{id}" href="{cancel_url}#menu-item-settings-{id}">{cancel}</a>				</div>

				<input class="menu-item-data-db-id" type="hidden" name="menu-item-db-id[{id}]" value="{id}" />
				<input class="menu-item-data-object-id" type="hidden" name="menu-item-object-id[{id}]" value="{object_id}" />
				<input class="menu-item-data-object" type="hidden" name="menu-item-object[{id}]" value="{object}" />
				<input class="menu-item-data-parent-id" type="hidden" name="menu-item-parent-id[{id}]" value="{parent}" />
				<input class="menu-item-data-position" type="hidden" name="menu-item-position[{id}]" value="{position}" />
				<input class="menu-item-data-type" type="hidden" name="menu-item-type[{id}]" value="{kind}" />
			</div><!-- .menu-item-settings-->
			<ul class="menu-item-transport"></ul>
