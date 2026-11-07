#!/usr/bin/env python3
"""Marks the texts of the baked pages in `crates/rustpress-admin/src/data/pages/*.json` and of the templates in `TEMPLATES`:
asks the reference (dev/up.sh) for each page with its texts marked (`dev/mu-plugins/rustpress-mark-texts.php`), lays the
marks over the baked page and writes it back. What was put in for the request by hand (`{{nonce:...}}`, `{{cards}}`) stays
where it is. `screens/template_text.rs` prints the result in the language of the site.

    python dev/bake-marks.py tools=tools.php import=import.php
    python dev/bake-marks.py --templates revision=853 attachment=300

Without arguments it marks all pages of `PAGES`. A mark that does not fit the baked page is reported and left out. The
texts of a baked page are English until they are marked. `--templates` needs a revision and an image of the reference.
A page of the reference does not show everything of a template: the marks of `image_editor_restore.tpl` and a few others
are written by hand, so look at the diff after a run."""
import base64
import difflib
import importlib.util
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, '..', 'crates', 'rustpress-admin', 'src', 'data', 'pages')
DATA = os.path.join(ROOT, '..')
SCREENS = os.path.join(DATA, '..', 'screens')
PAGES = {
    'tools': 'tools.php',
    'import': 'import.php',
    'export': 'export.php',
    'options-privacy': 'options-privacy.php',
    'privacy-policy-guide': 'options-privacy.php?tab=policyguide',
    'options-permalink': 'options-permalink.php',
    'plugins': 'plugins.php',
    'themes': 'themes.php',
    'site-health': 'site-health.php',
    'plugin-install-upload': 'plugin-install.php?tab=upload',
    'theme-install-upload': 'theme-install.php?upload',
    'about': 'about.php',
    'credits': 'credits.php',
    'freedoms': 'freedoms.php',
    'privacy': 'privacy.php',
    'contribute': 'contribute.php',
    'update-core': 'update-core.php',
    'attachment-edit': 'post.php?post=300&action=edit',
}

# The file of a template and the pages it was made from.
TEMPLATES = [
    (os.path.join(DATA, 'revision_templates.html'), 'revision.php?revision={revision}'),
    (os.path.join(DATA, 'app_passwords_section.tpl'), 'profile.php'),
    (os.path.join(DATA, 'app_passwords_templates.tpl'), 'profile.php'),
    (os.path.join(DATA, 'attachment_edit.tpl'), 'post.php?post={attachment}&action=edit'),
    (os.path.join(DATA, 'image_editor.tpl'), 'post.php?post={attachment}&action=edit&image-editor'),
    # The form differs between posts and pages; the template has the fields of both.
    (os.path.join(SCREENS, 'inline_edit.tpl'), 'edit.php', 'edit.php?post_type=page'),
]

spec = importlib.util.spec_from_file_location('bake', os.path.join(HERE, 'bake-pages.py'))
bake = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bake)


def fetch(page):
    """The page, with the header that asks for marks."""
    import http.cookiejar
    import urllib.parse
    import urllib.request
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    port = bake.PORT
    opener.open(f'http://127.0.0.1:{port}/wp-login.php').read()
    data = urllib.parse.urlencode({'log': 'rpadmin', 'pwd': 'rustpress-dev', 'testcookie': '1'}).encode()
    opener.open(f'http://127.0.0.1:{port}/wp-login.php', data).read()
    opener.addheaders = [('X-Rustpress-Marks', '1')]
    return opener.open(f'http://127.0.0.1:{port}/wp-admin/{page}').read().decode('utf-8')


def addresses(marked):
    """The addresses that went through `__()` get their marks: `rustpress-mark-texts.php` could only hang the message on them."""

    def mark(match):
        url = match.group(1)
        context, message = base64.b64decode(match.group(2).translate(str.maketrans('-_.', '+/='))).decode('utf-8').split('\ue003', 1)
        pattern = '(.*)'.join(re.escape(part) for part in re.split(r'%(?:\d+\$)?s', message))
        pieces = []
        found = re.fullmatch(pattern, url)
        at = 0
        for index, conversion in enumerate(re.finditer(r'%(?:\d+\$)?s', message)):
            pieces.append(message[at:conversion.start()])
            pieces.append(f'\ue004{conversion.group(0)}\ue006{found.group(index + 1)}\ue005')
            at = conversion.end()
        pieces.append(message[at:])
        return f'\ue001r\ue003{context}\ue003' + ''.join(pieces) + '\ue002'

    return re.sub(r'(https?://[^\s"\'<>#\ue000-\ue008]*)#rustpress-text-([\w.-]+)', mark, marked)


def spans(marked):
    """The page without marks and its marks: `[group, start, stop, head, tail, order of the head, order of the tail]`. A
    group is an outermost mark with what is nested in it."""
    marked = addresses(marked)
    plain = []
    at = 0
    points = []
    stack = []
    order = 0
    i = 0
    while i < len(marked):
        c = marked[i]
        if c in '':
            # The head of a text goes up to its second ``, of a conversion up to ``.
            end = marked.index('', marked.index('', i) + 1) if c == '' else marked.index('', i)
            order += 1
            point = [stack[0][0] if stack else len(points), at, None, marked[i:end + 1], None, order, None]
            stack.append(point)
            points.append(point)
            i = end + 1
        elif c in '':
            order += 1
            point = stack.pop()
            point[2], point[4], point[6] = at, c, order
            i += 1
        else:
            plain.append(c)
            at += 1
            i += 1
    return ''.join(plain), points


def tokens(text):
    return re.findall(r'\w+|\W', text)


def overlay(marked, target, crop=False):
    """`target` with the marks of `marked`, which is the same page but for what the hand changed. The second value counts
    the marks that had no place."""
    plain, points = spans(marked)
    # A template is a small part of its page.
    if crop:
        lines = [line.strip() for line in target.split('\n') if line.strip() and '{{' not in line and '⟦' not in line and len(line.strip()) > 12]
        first = plain.find(lines[0]) if lines else -1
        last = plain.rfind(lines[-1]) if lines else -1
        if first >= 0 and last >= first:
            first, last = max(0, first - 400), last + len(lines[-1]) + 400
            plain = plain[first:last]
            points = [[p[0], p[1] - first, p[2] - first, *p[3:]] for p in points if p[2] is not None and p[1] >= first and p[2] <= last]
    a, b = tokens(plain), tokens(target)
    offsets_a = [0]
    for t in a:
        offsets_a.append(offsets_a[-1] + len(t))
    offsets_b = [0]
    for t in b:
        offsets_b.append(offsets_b[-1] + len(t))
    blocks = []
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes():
        blocks.append((tag, offsets_a[i1], offsets_a[i2], offsets_b[j1], offsets_b[j2]))

    def place(position, closing):
        """Between two parts a text opens in the second and closes in the first; inside a changed part it has no place."""
        for tag, x1, x2, y1, y2 in blocks:
            if (x1 < position <= x2) if closing else (x1 <= position < x2):
                if tag == 'equal':
                    return y1 + position - x1
                if position == x1:
                    return y1
                return y2 if position == x2 else None
        return len(target) if position == len(plain) else None

    def same(members, spots):
        """Whether the place holds the text of the mark, but for the values in it."""
        here, there = members[0][1], spots[0][0]
        for point, (at, to) in zip(members[1:], spots[1:]):
            if point[1] < here:
                continue
            if plain[here:point[1]] != target[there:at]:
                return False
            here, there = point[2], to
        return plain[here:members[0][2]] == target[there:spots[0][1]]

    groups = {}
    for point in points:
        groups.setdefault(point[0], []).append(point)

    inserts = []
    placed = []
    moved = []

    def accept(members, spots):
        for point, (at, to) in zip(members, spots):
            inserts.append((at, point[5], point[3]))
            inserts.append((to, point[6], point[4]))
        placed.append(spots[0])

    for members in groups.values():
        spots = [(place(p[1], False), place(p[2], True)) for p in members]
        outer = spots[0]
        fits = all(at is not None and to is not None and to >= at for at, to in spots)
        if fits and same(members, spots):
            accept(members, spots)
        else:
            moved.append(members)

    # The hand may have moved a text: the same text elsewhere, a whole text of its element, outside of the other marks.
    missed = 0
    for members in moved:
        start, stop = members[0][1], members[0][2]
        text = plain[start:stop]
        spot = None
        for m in re.finditer(re.escape(text), target) if len(text) > 2 else ():
            before, after = target[m.start() - 1:m.start()], target[m.end():m.end() + 1]
            if before not in ('', '>', '"', "'", '\t', '\n') or after not in ('', '<', '"', "'", '\t', '\n'):
                continue
            if not any(x < m.end() and m.start() < y for x, y in placed):
                spot = m.start()
                break
        if spot is None:
            missed += 1
            print('  no place:', text[:90].replace('\n', ' '))
            continue
        accept(members, [(p[1] - start + spot, p[2] - start + spot) for p in members])

    inserts.sort(key=lambda m: (m[0], m[1]))
    out = []
    at = 0
    for position, _, text in inserts:
        out.append(target[at:position])
        out.append(text)
        at = position
    out.append(target[at:])
    return ''.join(out), missed


def readable(text):
    for raw, shown in (('', '⟦'), ('', '⟧'), ('', '¦'), ('', '⟪'), ('', '⟫'), ('', '¦'), ('', '¶'), ('', '％')):
        text = text.replace(raw, shown)
    return text


def unmarked(text):
    """A page that may hold the marks of an earlier run."""
    text = re.sub(r'⟦[^¦]*¦[^¦]*¦', '', text)
    text = re.sub(r'⟪[^¦]*¦', '', text)
    return text.replace('⟧', '').replace('⟫', '')


def templates(ids):
    """Marks the templates of `TEMPLATES`."""
    site = f'http://localhost:{bake.PORT}'
    for path, *pages in TEMPLATES:
        parts = [fetch(page.format(**ids)).replace(site, '{{site}}') for page in pages]
        # Of several pages only the form counts, or the text between them would be part of the template.
        html = parts[0] if len(parts) == 1 else ''.join(re.search(r'<table style="display: none">.*?</table>', part, re.S).group(0) for part in parts)
        with open(path, encoding='utf-8', newline='') as f:
            target = f.read().replace('\r\n', '\n')
        marked, missed = overlay(html, unmarked(target), crop=True)
        with open(path, 'w', encoding='utf-8', newline='') as f:
            f.write(readable(marked))
        print(os.path.basename(path), readable(marked).count('⟦'), 'marks,', missed, 'without a place')


def main():
    site = f'http://localhost:{bake.PORT}'
    if sys.argv[1:2] == ['--templates']:
        templates(dict(arg.split('=', 1) for arg in sys.argv[2:]))
        return
    pages = dict(arg.split('=', 1) for arg in sys.argv[1:]) or PAGES
    for name, page in pages.items():
        path = os.path.join(ROOT, name + '.json')
        data = json.load(open(path, encoding='utf-8'))
        html = fetch(page)
        help_ = bake.help_of(html)
        content = bake.content_of(html).replace(site, '{{site}}')
        data['content'], missed = overlay(content, unmarked(data['content']))
        tabs = {tab['id']: tab for tab in help_['tabs']}
        for tab in data['help']['tabs']:
            marked = tabs.get(tab['id'])
            if marked is None:
                continue
            for key in ('title', 'content'):
                tab[key], more = overlay(marked[key].replace(site, '{{site}}'), unmarked(tab[key]))
                missed += more
        data['help']['sidebar'], more = overlay(help_['sidebar'].replace(site, '{{site}}'), unmarked(data['help']['sidebar']))
        missed += more
        with open(path, 'w', encoding='utf-8', newline='\n') as f:
            json.dump(json.loads(readable(json.dumps(data, ensure_ascii=False))), f, ensure_ascii=False, indent=1)
        print(name, readable(data['content']).count('⟦'), 'marks,', missed, 'without a place')


if __name__ == '__main__':
    main()
