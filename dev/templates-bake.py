#!/usr/bin/env python3
"""Bakes the Underscore templates of the media modal and the widgets from the reference (dev/up.sh) into
`crates/rustpress-admin/src/data/`, with their texts marked: `⟦kind¦context¦text⟧`, where the text may hold
`⟪conversion¦value⟫`. `screens/template_text.rs` prints them in the language of the site.

    python dev/templates-bake.py"""
import base64
import json
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'crates', 'rustpress-admin', 'src', 'data')


def wsl_path(path):
    return subprocess.run(['wsl', 'wslpath', '-a', path.replace('\\', '/')], capture_output=True, text=True).stdout.strip()


def readable(text):
    for raw, shown in (('\ue001', '⟦'), ('\ue002', '⟧'), ('\ue003', '¦'), ('\ue004', '⟪'), ('\ue005', '⟫'), ('\ue006', '¦')):
        text = text.replace(raw, shown)
    return text


script = f"podman cp '{wsl_path(os.path.join(HERE, 'templates-bake.php'))}' rustpress-wp:/tmp/rp-bake.php >/dev/null && podman exec rustpress-wp php -d display_errors=0 /tmp/rp-bake.php"
output = subprocess.run(['wsl', 'bash', '-c', script], capture_output=True, encoding='utf-8', errors='replace')
data = json.loads(output.stdout[output.stdout.index('{"media"'):])
media = readable(base64.b64decode(data['media']).decode('utf-8'))
widgets = readable(base64.b64decode(data['widgets']).decode('utf-8'))
# `wp_admin_notice()` runs `wp_kses_post()`, which takes the marks around the address for a protocol.
widgets = widgets.replace('<a href="8080/wp-admin/upload.php⟫">', '<a href="⟪%s¦http://localhost:8080/wp-admin/upload.php⟫">')
with open(os.path.join(OUT, 'media_templates.html'), 'w', encoding='utf-8', newline='') as f:
    f.write(media)
with open(os.path.join(OUT, 'widgets_templates.html'), 'w', encoding='utf-8', newline='') as f:
    f.write(widgets)
print(len(media), len(widgets), media.count('⟦'), widgets.count('⟦'))
