#!/usr/bin/env python3
"""Erases the comments of two test addresses, one through the reference (:8080) and one through RustPress (:3001), and
compares what is left in the database. The test comments and requests are removed at the end.

    python dev/privacy-erase-compare.py"""
import importlib.util
import os
import re
import subprocess

spec = importlib.util.spec_from_file_location('pc', os.path.join(os.path.dirname(__file__), 'privacy-compare.py'))
pc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pc)

HERE = os.path.dirname(os.path.abspath(__file__))


def sql(statement):
    out = subprocess.run(['powershell', '-File', os.path.join(HERE, 'sql.ps1'), 'wordpress', statement], capture_output=True, text=True, encoding='utf-8')
    return out.stdout.strip()


def add_comment(email, ip):
    sql(f"INSERT INTO wp_comments (comment_post_ID, comment_author, comment_author_email, comment_author_url, comment_author_IP, comment_date, comment_date_gmt, comment_content, comment_approved, comment_agent, comment_type, comment_parent, user_id) VALUES (1, 'Erase Test', '{email}', 'http://example.com', '{ip}', NOW(), UTC_TIMESTAMP(), 'erase test', '1', 'Agent/1.0', 'comment', 0, 0)")


def main():
    rp, ref = pc.Site(3001), pc.Site(8080)
    results = {}
    emails = {8080: 'erase-ref@example.com', 3001: 'erase-rp@example.com'}
    ips = {8080: '203.0.113.77', 3001: '203.0.113.77'}
    try:
        for site in (ref, rp):
            add_comment(emails[site.port], ips[site.port])
            page = rp.get('/wp-admin/erase-personal-data.php')
            nonce = re.search(r'name="_wpnonce" value="([0-9a-f]+)"', page).group(1)
            rp.post('/wp-admin/erase-personal-data.php', {'_wpnonce': nonce, 'action': 'add_remove_personal_data_request', 'type_of_action': 'remove_personal_data', 'username_or_email_for_privacy_request': emails[site.port]})
            ids = [int(i) for i in re.findall(r'id="request-(\d+)"', rp.get('/wp-admin/erase-personal-data.php'))]
            request_id = max(ids)
            page = site.get('/wp-admin/erase-personal-data.php')
            token = re.search(rf'data-request-id="{request_id}" data-nonce="([0-9a-f]+)"', page).group(1)
            status, text = site.post('/wp-admin/admin-ajax.php', {'action': 'wp-privacy-erase-personal-data', 'id': request_id, 'security': token, 'eraser': 1, 'page': 1})
            print(site.port, status, text)
            row = sql(f"SELECT comment_author, comment_author_email, comment_author_url, comment_author_IP, comment_agent, user_id FROM wp_comments WHERE comment_content = 'erase test' AND comment_author_IP LIKE '203.0.113.%' ORDER BY comment_ID DESC LIMIT 1")
            status_row = sql(f"SELECT post_status FROM wp_posts WHERE ID = {request_id}")
            results[site.port] = (text, row, status_row)
            sql("DELETE FROM wp_comments WHERE comment_content = 'erase test'")
            sql(f"DELETE FROM wp_postmeta WHERE post_id = {request_id}")
            sql(f"DELETE FROM wp_posts WHERE ID = {request_id}")
        same = results[8080] == results[3001]
        print('identical' if same else 'DIFFERENT')
        if not same:
            print(results[8080])
            print(results[3001])
    finally:
        sql("DELETE FROM wp_comments WHERE comment_content = 'erase test'")
        sql("DELETE FROM wp_posts WHERE post_type = 'user_request'")


if __name__ == '__main__':
    main()
