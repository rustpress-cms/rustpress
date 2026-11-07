#!/usr/bin/env python3
"""Bakes the Underscore templates of `wp-admin/customize.php` from the reference (dev/up.sh) into
`crates/rustpress-admin/src/data/customize_*.html`, with their texts marked: `⟦kind¦context¦text⟧`, where the text may
hold `⟪conversion¦value⟫`. `screens/template_text.rs` prints them in the language of the site.

`customize_widget_forms.html` holds the form of each core widget as `WP_Widget::form_callback( -1 )` prints it, each
after `⟨id base⟩`, without the menu widget's.

What depends on the request is left to RustPress: the Screen Options of the menus panel are `{{screen_options}}`, the
address of the administration `{{admin}}` and of the site `{{site}}`; see `replacements` for the rest.

    python dev/customize-bake.py"""
import base64
import importlib.util
import json
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'crates', 'rustpress-admin', 'src', 'data')


def wsl_path(path):
    return subprocess.run(['wsl', 'wslpath', '-a', path.replace('\\', '/')], capture_output=True, text=True).stdout.strip()


spec = importlib.util.spec_from_file_location('marks', os.path.join(HERE, 'bake-marks.py'))
marks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(marks)
readable = lambda text: marks.readable(marks.addresses(text))

script = f"podman cp '{wsl_path(os.path.join(HERE, 'customize-bake.php'))}' rustpress-wp:/tmp/rp-customize-bake.php >/dev/null && podman exec rustpress-wp php -d display_errors=0 /tmp/rp-customize-bake.php"
output = subprocess.run(['wsl', 'bash', '-c', script], capture_output=True, encoding='utf-8', errors='replace')
data = json.loads(output.stdout[output.stdout.index('{"templates"'):])


def json_marks(text):
    """A text that went through `wp_json_encode()` is a JSON string with its mark escaped: the mark gets the kind `j`."""

    def mark(match):
        plain = json.loads(match.group(0))
        assert plain.startswith('') and plain.endswith(''), plain
        return 'j' + plain[2:]

    return re.sub(r'"\\ue001(?:[^"\\]|\\.)*?\\ue002"', mark, text)


forms = json.loads(base64.b64decode(data.pop('forms')).decode('utf-8'))
parts = {name: readable(json_marks(base64.b64decode(value).decode('utf-8'))) for name, value in data.items()}
# The forms of the widgets, one after the other: `⟨id base⟩` starts each.
parts['widget_forms'] = ''.join(f'⟨{id_base}⟩' + readable(form) for id_base, form in forms.items())
# The taxonomies of the tag cloud were registered before the texts were marked: RustPress lists them.
parts['widget_forms'], count = re.subn(r'(<select class="widefat" id="widget-tag_cloud-__i__-taxonomy"[^>]*>\n).*?(\t*</select>)', r'\1{{tag_cloud_taxonomies}}\2', parts['widget_forms'], flags=re.S)
assert count == 1, 'the taxonomies of the tag cloud were not found'
# What depends on the site: the Screen Options of the menus panel, the date control (its data, the timezone and the
# order of the date fields), the menu locations of the theme and the name of the site in the preview of the icon.
replacements = [
    (r"<form id='adv-settings' method='post'>.*?</form>\n", '{{screen_options}}'),
    (r'(<script type="text/html" id="tmpl-customize-control-date_time-content">).*?(</script>)', r'\1{{date_time}}\2'),
    (r'(<script type="text/html" id="tmpl-customize-control-nav_menu_locations-content">).*?(</script>)', r'\1{{menu_locations}}\2'),
    (r"<# print\( 'RustPress Test' \) #>", "<# print( '{{blogname_js}}' ) #>"),
]
for pattern, replacement in replacements:
    parts['templates'], count = re.subn(pattern, replacement, parts['templates'], flags=re.S)
    assert count == 1, f'{pattern} was not found'
for name, text in parts.items():
    text = text.replace('http://localhost:8080/wp-admin/', '{{admin}}').replace('http://localhost:8080', '{{site}}')
    assert 'localhost:8080' not in text, f'{name} holds an address of the reference'
    with open(os.path.join(OUT, f'customize_{name}.html'), 'w', encoding='utf-8', newline='') as f:
        f.write(text)
    print(name, len(text), text.count('⟦'))
