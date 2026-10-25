#!/usr/bin/env python3
"""Checks `rustpress_core::text_diff::wp_text_diff` against the reference WordPress.

    python dev/text-diff-check.py [seed] [count]       writes $TEMP/diff_cases.json and $TEMP/diff_php.txt
    (PowerShell)  Get-Content -Raw $env:TEMP\\diff_php.txt | wsl.exe -d Debian -- bash dev/wp-eval.sh  > $env:TEMP\\diff_ref.b64
    python dev/text-diff-check.py --decode              turns diff_ref.b64 into diff_ref.json
    cargo test -p rustpress-core --test text_diff_live -- --nocapture
"""
import base64
import json
import os
import random
import sys

T = os.environ['TEMP']


def generate(seed, count):
    random.seed(seed)
    vocab = 'the quick brown fox jumps over lazy dog hello world <b>bold</b> & ampersand "quote" it\'s café naïve ünï 123 foo_bar ,.; apple banana cherry 0 1 1e3 1000'.split()

    def line():
        return ' '.join(random.choice(vocab) for _ in range(random.randint(0, 9)))

    def mutate(lines):
        out = list(lines)
        for _ in range(random.randint(0, 6)):
            r = random.random()
            if r < 0.3 and out:
                out.pop(random.randrange(len(out)))
            elif r < 0.6:
                out.insert(random.randint(0, len(out)), line())
            elif out:
                i = random.randrange(len(out))
                ws = out[i].split(' ')
                ws[random.randrange(len(ws))] = random.choice(vocab)
                out[i] = ' '.join(ws)
            if random.random() < 0.1 and out:
                out.append(out[random.randrange(len(out))])
        return out

    cases = []
    for _ in range(count):
        a = [line() for _ in range(random.randint(0, 14))]
        cases.append(['\n'.join(a), '\n'.join(mutate(a))])
    json.dump(cases, open(T + '/diff_cases.json', 'w', encoding='utf-8'), ensure_ascii=False)
    b64 = base64.b64encode(json.dumps(cases, ensure_ascii=False).encode('utf-8')).decode()
    php = "$cases=json_decode(base64_decode('%s'),true); $out=array(); foreach($cases as $c){ $out[]=wp_text_diff($c[0],$c[1],array('show_split_view'=>true,'title_left'=>'Removed','title_right'=>'Added')); } echo base64_encode(json_encode($out));" % b64
    open(T + '/diff_php.txt', 'w').write(php)
    print(len(cases), 'cases')


def decode():
    data = json.loads(base64.b64decode(open(T + '/diff_ref.b64').read().strip()).decode('utf-8'))
    json.dump(data, open(T + '/diff_ref.json', 'w', encoding='utf-8'), ensure_ascii=False)
    print(len(data), 'results')


if '--decode' in sys.argv:
    decode()
else:
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    generate(int(args[0]) if args else 1, int(args[1]) if len(args) > 1 else 80)
