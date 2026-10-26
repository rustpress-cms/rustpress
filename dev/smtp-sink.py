#!/usr/bin/env python3
"""A mail server that keeps what it is given: for testing the e-mail of RustPress.

    python dev/smtp-sink.py [port]        # listens on 127.0.0.1:2525; every message is appended to $TEMP/smtp-sink.log

Start RustPress with RUSTPRESS_SMTP_URL=smtp://127.0.0.1:2525 (and RUSTPRESS_MAIL_FROM if the sender matters)."""
import asyncio
import os
import sys

LOG = os.path.join(os.environ.get('TEMP', '.'), 'smtp-sink.log')


async def session(reader, writer):
    writer.write(b'220 sink ESMTP\r\n')
    sender, recipients, data = '', [], []
    in_data = False
    while True:
        line = await reader.readline()
        if not line:
            break
        text = line.decode('utf-8', 'replace')
        if in_data:
            if text == '.\r\n':
                in_data = False
                with open(LOG, 'a', encoding='utf-8', newline='\n') as out:
                    out.write('=== FROM %s TO %s\n%s\n' % (sender, ','.join(recipients), ''.join(data)))
                writer.write(b'250 queued\r\n')
                data = []
            else:
                data.append((text[1:] if text.startswith('..') else text).replace('\r\n', '\n'))
            await writer.drain()
            continue
        command = text.strip().upper()
        if command.startswith('EHLO') or command.startswith('HELO'):
            writer.write(b'250-sink\r\n250 8BITMIME\r\n')
        elif command.startswith('MAIL FROM'):
            sender = text.split(':', 1)[1].strip()
            recipients = []
            writer.write(b'250 ok\r\n')
        elif command.startswith('RCPT TO'):
            recipients.append(text.split(':', 1)[1].strip())
            writer.write(b'250 ok\r\n')
        elif command == 'DATA':
            in_data = True
            writer.write(b'354 go ahead\r\n')
        elif command == 'QUIT':
            writer.write(b'221 bye\r\n')
            await writer.drain()
            break
        else:
            writer.write(b'250 ok\r\n')
        await writer.drain()
    writer.close()


async def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 2525
    server = await asyncio.start_server(session, '127.0.0.1', port)
    print('listening on 127.0.0.1:%d, writing to %s' % (port, LOG), flush=True)
    async with server:
        await server.serve_forever()


asyncio.run(main())
