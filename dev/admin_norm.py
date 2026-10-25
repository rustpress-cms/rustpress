#!/usr/bin/env python3
"""Normalizes an administration page for dev/admin-compare.sh: the frame and the content only, one tag per
line, nonces, ports and times neutralized, and the parts RustPress leaves out (help tabs, the command palette,
screen options) removed from both sides."""
import re
import sys

text = sys.stdin.buffer.read().decode("utf-8", "replace").replace("\r", "")
start = text.find('<div id="wpwrap">')
end = text.find('<div id="wpfooter"')
if start >= 0:
    text = text[start : end if end >= 0 else None]
# The command palette item of the toolbar.
text = re.sub(r"<li role='group' id='wp-admin-bar-command-palette'.*?</li>", "", text, flags=re.S)
# Help tabs and screen options, up to the beginning of the page proper.
text = re.sub(r'<div id="screen-meta".*?<div id="screen-meta-links">.*?</div>\s*</div>\s*', "", text, flags=re.S)
text = re.sub(r"<script.*?</script>", "<script/>", text, flags=re.S)
text = text.replace("localhost:3001", "localhost:8080")
text = re.sub(r"(nonce|_wpnonce)([\"' =:]+(?:value=\")?)[0-9a-f]{10}", r"\1\2N", text)
text = re.sub(r"_wpnonce=[0-9a-f]{10}", "_wpnonce=N", text)
text = re.sub(r'(name="(?:_wpnonce[\w-]*|_inline_edit|_ajax_nonce[\w-]*|[\w-]*nonce)" value=")[0-9a-f]{10}', r"\g<1>N", text)
text = re.sub(r'(id="(?:_wpnonce[\w-]*|_inline_edit|[\w-]*nonce)" name="[\w-]+" value=")[0-9a-f]{10}', r"\g<1>N", text)
text = re.sub(r'(name="_wp_unfiltered_html_comment" value=")[0-9a-f]{10}', r"\g<1>N", text)
text = re.sub(r"<code>\d{4}-\d{2}-\d{2} [\d:]{8}</code>", "<code>TIME</code>", text)
# The current time in the date fields, and `antispambot()` (random per request).
text = re.sub(r'(id="(?:ss|mn|hh)" name="(?:ss|mn|hh)" value=")\d\d', r"\g<1>00", text)


def _unmangle(match):
    import html

    return html.unescape(match.group(0))


text = re.sub(r'<a href="mailto:[^"]*">[^<]*</a>', _unmangle, text)
text = text.replace("><", ">\n<")
text = "\n".join(line.rstrip() for line in text.split("\n") if line.strip())
sys.stdout.buffer.write((text + "\n").encode("utf-8"))
