#!/usr/bin/env python3
"""Makes the golden file of the comment filters: inputs and what the reference WordPress answers to
`apply_filters( 'pre_comment_content', ... )` (kses for a visitor, `rel="ugc"`), for
crates/rustpress-core/tests/kses_golden.rs.

    python dev/kses-golden.py

Needs the reference container (WSL, podman)."""
import json
import os
import subprocess
import sys

CASES = [
    'plain text',
    'Tom & Jerry',
    'a < b and c > d',
    '<b>bold</b> <i>italic</i> <strong>x</strong> <em>y</em> <u>underline</u>',
    '<a href="http://example.com/">link</a>',
    "<a href='http://example.com/' title='Titel'>link</a>",
    '<a href=http://example.com/>unquoted</a>',
    '<a href="javascript:alert(1)">js</a>',
    '<a href="JaVaScRiPt:alert(1)">js</a>',
    '<a href="java&#x09;script:alert(1)">tab</a>',
    '<a href="&#106;avascript:alert(1)">entity</a>',
    '<a href="jav&#x61;script:alert(1)">hex</a>',
    '<a href=" javascript:alert(1)">space</a>',
    '<a href="data:text/html;base64,PHNjcmlwdD4=">data</a>',
    '<a href="mailto:me@example.com">mail</a>',
    '<a href="tel:+491234">tel</a>',
    '<a href="/relative/path?x=1&y=2">rel</a>',
    '<a href="#anchor">anchor</a>',
    '<a href="http://localhost:8080/page/">internal</a>',
    '<a href="http://example.com/" rel="friend">has rel</a>',
    '<a href="http://example.com/" onclick="alert(1)" class="c" style="color:red">attrs</a>',
    '<a href="http://example.com/" title="a &amp; b &lt; c &quot;q&quot;">entities in title</a>',
    '<a href="http://example.com/?a=1&b=2&amp;c=3">amp</a>',
    '<a href>empty</a>',
    '<a href="">empty value</a>',
    '<a HREF="http://example.com/" TITLE="x">upper</a>',
    '<A href="http://example.com/">Upper tag</A>',
    '<a href="http://example.com/" href="http://evil.example/">duplicate</a>',
    '<a title="no href">t</a>',
    '<abbr title="x">y</abbr> <acronym title="z">w</acronym> <cite>c</cite> <code>code</code> <q cite="http://q.example/">q</q>',
    '<blockquote cite="http://b.example/">quote</blockquote>',
    '<del datetime="2020-01-01">old</del> <s>s</s> <strike>st</strike>',
    '<br> <br/> <br />',
    '<img src="http://example.com/a.png" alt="a">',
    '<script>alert(1)</script>',
    '<script src="http://evil.example/x.js"></script>',
    '<style>body{display:none}</style>',
    '<iframe src="http://example.com/"></iframe>',
    '<div class="x">div</div> <p>para</p> <span>span</span> <span class="wp-note-mention user-5 other">m</span>',
    '<h1>head</h1> <ul><li>item</li></ul>',
    '<!-- a comment --> text',
    '<!-- <script>x</script> -->',
    '<!--a--b-->',
    '</b>stray close',
    '<b>unclosed',
    '<b onmouseover=alert(1)>x',
    '<b/onmouseover=alert(1)>x</b>',
    '<a/href="http://example.com/">slash</a>',
    '<a href="http://example.com/"onclick="x">nospace</a>',
    '<a href="http://example.com/\'>quote</a>',
    '<a href="foo>bar">gt in value</a>',
    '<a href=\'foo\'>sq</a>',
    '< b>space</b>',
    '<b >trailing space</b >',
    '<<b>>double</b>',
    '&nbsp;&copy;&amp;&lt;&gt;&quot;&apos;&hellip;&notanentity;&#65;&#x41;&#0;&#xD800;&#128;',
    '&#38; &#x26; &#060; &#x3c;',
    '&amp;amp; &amp',
    'line1\nline2\r\nline3',
    "null\x00byte and \x01 control",
    '\\"escaped quotes\\" <a href=\\"http://example.com/\\">x</a>',
    'ünïcödé <b>bold</b> 日本語 😀',
    '<a href="http://example.com/日本語?q=é">unicode url</a>',
    '<a href="http://example.com/" title="é &eacute; &#233;">ent</a>',
    '<q cite="javascript:x">q</q>',
    '<blockquote cite="  http://x.example/  ">q</blockquote>',
    '<a href="feed:javascript:alert(1)">feed</a>',
    '<a href="feed:feed:feed:feed:http://x.example/">feeds</a>',
    '<a href="http://example.com" title=\'a"b\'>mixed quotes</a>',
    '<a href="http://example.com" title="a\'b">apos</a>',
    '<a href="http://example.com" data-x="1" aria-label="l">data</a>',
    '<a href=http://example.com/a&b>unquoted amp</a>',
    '<a href="http://example.com/&#x3a;">colon</a>',
    '<a href="&colon;javascript">colon name</a>',
    '<a href="javascript&colon;alert(1)">colon name 2</a>',
    '<a href="javascript&#58;alert(1)">colon dec</a>',
    '<a href="javascript&#x3a;alert(1)">colon hex</a>',
    '<a href="http://example.com" title=>missing value</a>',
    '<a href="http://example.com" =foo>weird</a>',
    '<a "x" href="http://example.com">quoted name</a>',
    '<b title="x">bold with attr</b>',
    '<em class="c" id="i">em</em>',
    'text with <b>nested <i>tags <a href="http://example.com/">here</a></i></b> end',
    '<a href="http://example.com/"><img src="x"></a>',
    '<a href="http://example.com/">a</a><a href="http://example.org/">b</a>',
    '<a   href="http://example.com/"   title="x"  >spaces</a>',
    '<a\nhref="http://example.com/"\ntitle="x">newlines</a>',
    '<a href="http://example.com/\n">newline in value</a>',
    '<a href="HTTP://EXAMPLE.COM/">upper scheme</a>',
    '<a href="ftp://example.com/f">ftp</a>',
    '<a href="irc://example.com/c">irc</a>',
    '<a href="ssh://example.com/">ssh</a>',
    '<a href="//example.com/x">protocol relative</a>',
    '<a href="http://example.com/?x=<script>">lt in value</a>',
    '<a href="http://example.com/" title="<b>x</b>">tags in title</a>',
]


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(here)
    php = '<?php\n$cases = json_decode(file_get_contents("/tmp/kses-cases.json"), true);\n$out = array();\nforeach ($cases as $c) { $out[] = array("in" => $c, "out" => wp_unslash(apply_filters("pre_comment_content", wp_slash($c)))); }\necho json_encode($out, JSON_UNESCAPED_UNICODE|JSON_UNESCAPED_SLASHES);\n'
    tmp = os.environ.get('TEMP', '.')
    os.makedirs(os.path.join(tmp, 'phpq'), exist_ok=True)
    open(os.path.join(tmp, 'phpq', 'kses.php'), 'w', encoding='utf-8', newline='\n').write(php)
    open(os.path.join(tmp, 'phpq', 'kses-cases.json'), 'w', encoding='utf-8', newline='\n').write(json.dumps(CASES))
    win = lambda p: '/mnt/' + p[0].lower() + p[2:].replace('\\', '/')
    script = (
        f'podman cp {win(os.path.join(tmp, "phpq", "kses-cases.json"))} rustpress-wp:/tmp/kses-cases.json && '
        f'bash /mnt/c/work/experimental/rustpress/dev/php.sh {win(os.path.join(tmp, "phpq", "kses.php"))}'
    )
    env = dict(os.environ, MSYS_NO_PATHCONV='1')
    result = subprocess.run(['wsl.exe', '-d', 'Debian', '--', 'bash', '-c', script], capture_output=True, text=True, encoding='utf-8', env=env)
    if result.returncode != 0:
        print(result.stderr)
        sys.exit(1)
    data = json.loads(result.stdout[result.stdout.index('['):])
    path = os.path.join(root, 'crates', 'rustpress-core', 'tests', 'data', 'kses_cases.json')
    open(path, 'w', encoding='utf-8', newline='\n').write(json.dumps(data, ensure_ascii=False, indent=1))
    print(len(data), 'cases written to', path)


if __name__ == '__main__':
    main()
