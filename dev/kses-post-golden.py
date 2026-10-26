#!/usr/bin/env python3
"""Makes the golden file of `wp_kses_post()`: inputs and what the reference WordPress answers, for
crates/rustpress-core/tests/kses_golden.rs (the post cases).

    python dev/kses-post-golden.py

Needs the reference container (WSL, podman)."""
import importlib.util
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('kg', os.path.join(HERE, 'kses-golden.py'))
kg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kg)

CASES = kg.CASES + [
    '<p class="x" id="i" style="color:red;behavior:url(x.htc)">p</p>',
    '<p style="background-image:url(javascript:alert(1))">p</p>',
    '<p style="background-image:url(\'https://example.org/a.png\');margin:0">p</p>',
    '<p style="width:calc(100% - 10px);color:expression(alert(1))">p</p>',
    '<p style="--my-var:var(--x);background:linear-gradient(red,blue)">p</p>',
    '<p style="color:red /* x */">p</p>',
    '<img src="https://example.org/a.png" alt="a" loading="lazy" width="10" height="10" onerror="alert(1)">',
    '<img/src=x/onerror=alert(1)>',
    '<img src=x onerror\n=alert(1)>',
    '<img src=x onerror =alert(1)>',
    '<a href=" javascript:alert(1)">x</a>',
    '<a href="&#106;avascript:alert(1)">x</a>',
    '<a href="java&#9;script:alert(1)">x</a>',
    '<a href="https://example.org/" download>d</a>',
    '<a href="https://example.org/" download="file.txt">d</a>',
    '<a href="https://example.org/" target="_blank" rel="noopener" data-id="5" data-x.y="1" DATA-Z="2" aria-label="l" role="button" tabindex="0">a</a>',
    '<svg onload="alert(1)"><circle r="1"/></svg>',
    '<math><mi>x</mi><annotation encoding="x">y</annotation></math>',
    '<details open ontoggle="alert(1)"><summary>s</summary>d</details>',
    '<form action="https://evil.example/"><input name="a"><button formaction="javascript:alert(1)">b</button></form>',
    '<object data="https://example.org/f.pdf" type="application/pdf"></object>',
    '<object data="https://example.org/f.swf" type="application/x-shockwave-flash"></object>',
    '<video src="https://example.org/v.mp4" controls autoplay muted playsinline poster="https://example.org/p.png"></video>',
    '<audio src="https://example.org/a.mp3" controls loop></audio>',
    '<iframe src="https://example.org/"></iframe>',
    '<table><thead><tr><th scope="col">h</th></tr></thead><tbody><tr><td colspan="2">d</td></tr></tbody></table>',
    '<figure class="wp-block-image"><img src="https://example.org/a.png"><figcaption>c</figcaption></figure>',
    '<h2 class="wp-block-heading" id="x">h</h2><ul class="wp-block-list"><li>a</li></ul>',
    '<div class="wp-block-group" style="padding:10px"><p>x</p></div>',
    '<span style="font-size:12px;text-decoration:underline">s</span>',
    '<time datetime="2020-01-01">t</time> <mark>m</mark> <kbd>k</kbd> <sub>1</sub><sup>2</sup>',
    '<textarea name="t">x</textarea><label for="i">l</label><meter value="1">m</meter><progress value="1">p</progress>',
    '<area shape="rect" coords="0,0,1,1" href="https://example.org/" alt="a"><map name="m"></map>',
    '<p hidden dir="rtl" lang="de" xml:lang="de" title="t">p</p>',
    '<ins datetime="2020-01-01" cite="https://example.org/">i</ins> <del datetime="2020-01-01">d</del>',
    '<font color="red" face="x" size="1">f</font> <big>b</big> <tt>t</tt>',
    '<search><input></search> <dialog open>d</dialog> <menu><li>x</li></menu>',
    '<a href="https://example.org/" style="color:red" onclick="x" onmouseover=y class="c">a</a>',
    '<!-- wp:paragraph --><p class="wp-block-paragraph">x</p><!-- /wp:paragraph -->',
    'İİ<b>Turkish</b>',
]


def main():
    root = os.path.dirname(HERE)
    php = '<?php\n$cases = json_decode(file_get_contents("/tmp/kses-post-cases.json"), true);\n$out = array();\nforeach ($cases as $c) { $out[] = array("in" => $c, "out" => wp_kses_post($c)); }\necho json_encode($out, JSON_UNESCAPED_UNICODE|JSON_UNESCAPED_SLASHES);\n'
    tmp = os.environ.get('TEMP', '.')
    os.makedirs(os.path.join(tmp, 'phpq'), exist_ok=True)
    open(os.path.join(tmp, 'phpq', 'kses-post.php'), 'w', encoding='utf-8', newline='\n').write(php)
    open(os.path.join(tmp, 'phpq', 'kses-post-cases.json'), 'w', encoding='utf-8', newline='\n').write(json.dumps(CASES))
    win = lambda p: '/mnt/' + p[0].lower() + p[2:].replace('\\', '/')
    script = (
        f'podman cp {win(os.path.join(tmp, "phpq", "kses-post-cases.json"))} rustpress-wp:/tmp/kses-post-cases.json && '
        f'bash /mnt/c/work/experimental/rustpress/dev/php.sh {win(os.path.join(tmp, "phpq", "kses-post.php"))}'
    )
    env = dict(os.environ, MSYS_NO_PATHCONV='1')
    result = subprocess.run(['wsl.exe', '-d', 'Debian', '--', 'bash', '-c', script], capture_output=True, text=True, encoding='utf-8', env=env)
    if result.returncode != 0:
        print(result.stderr)
        sys.exit(1)
    data = json.loads(result.stdout[result.stdout.index('['):])
    path = os.path.join(root, 'crates', 'rustpress-core', 'tests', 'data', 'kses_post_cases.json')
    open(path, 'w', encoding='utf-8', newline='\n').write(json.dumps(data, ensure_ascii=False, indent=1))
    print(len(data), 'cases written to', path)


if __name__ == '__main__':
    main()
