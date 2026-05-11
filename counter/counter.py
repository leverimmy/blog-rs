#!/usr/bin/env python3
"""Lightweight page view counter API for blog-rs.

Uses SQLite for storage, runs behind Nginx reverse proxy.
Endpoints:
  POST /api/count  — increment and return view count for a URL
  GET  /api/counts — return all counts sorted by views (top N via ?top=N)
"""

import json
import os
import sqlite3
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn

DB_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "counter.db")
LISTEN_HOST = "127.0.0.1"
LISTEN_PORT = 8123


def get_db():
    conn = sqlite3.connect(DB_PATH)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS page_views "
        "(url TEXT PRIMARY KEY, views INTEGER DEFAULT 0, title TEXT DEFAULT '')"
    )
    conn.execute("PRAGMA journal_mode=WAL")
    return conn


class CounterHandler(BaseHTTPRequestHandler):
    def _send_json(self, status, data):
        body = json.dumps(data, ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path.startswith("/api/counts"):
            top = 0
            if "top=" in self.path:
                try:
                    top = int(self.path.split("top=")[1].split("&")[0])
                except (ValueError, IndexError):
                    top = 0

            conn = get_db()
            if top > 0:
                rows = conn.execute(
                    "SELECT url, views, title FROM page_views ORDER BY views DESC LIMIT ?",
                    (top,),
                ).fetchall()
            else:
                rows = conn.execute(
                    "SELECT url, views, title FROM page_views ORDER BY views DESC"
                ).fetchall()
            conn.close()

            counts = [{"url": r[0], "views": r[1], "title": r[2]} for r in rows]
            self._send_json(200, {"counts": counts})
        else:
            self._send_json(404, {"error": "not found"})

    def do_POST(self):
        if self.path == "/api/count":
            try:
                length = int(self.headers.get("Content-Length", 0))
                body = json.loads(self.rfile.read(length)) if length else {}
            except (json.JSONDecodeError, ValueError):
                self._send_json(400, {"error": "invalid json"})
                return

            url = body.get("url", "").strip()
            if not url:
                self._send_json(400, {"error": "missing url"})
                return

            title = body.get("title", "").strip()

            conn = get_db()
            conn.execute(
                "INSERT INTO page_views (url, views, title) VALUES (?, 1, ?) "
                "ON CONFLICT(url) DO UPDATE SET views = views + 1, "
                "title = CASE WHEN ? != '' THEN ? ELSE title END",
                (url, title, title, title),
            )
            row = conn.execute(
                "SELECT views FROM page_views WHERE url = ?", (url,)
            ).fetchone()
            conn.commit()
            conn.close()

            self._send_json(200, {"url": url, "views": row[0]})
        else:
            self._send_json(404, {"error": "not found"})

    def log_message(self, format, *args):
        sys.stderr.write(f"[counter] {args[0]}\n")


class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else LISTEN_PORT
    server = ThreadedHTTPServer((LISTEN_HOST, port), CounterHandler)
    print(f"Counter API running on http://{LISTEN_HOST}:{port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down.")
        server.shutdown()
