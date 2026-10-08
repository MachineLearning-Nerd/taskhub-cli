#!/usr/bin/env python3
"""A local HTTP proxy for the end-to-end run that holds back one response.

The first request whose method and path match is forwarded at once, so the server commits it, but its
response is held for DELAY seconds. That gives the run time to kill the CLI after a write was sent and
before the reply arrived. Every other request passes straight through.

Usage: e2e-delay-proxy.py UPSTREAM METHOD PATH_REGEX DELAY
Prints the port it listens on (127.0.0.1), then serves until killed. Each request is logged to stderr as
"METHOD PATH", so a run can count requests per command.
"""

import http.client
import re
import sys
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

UPSTREAM = urllib.parse.urlsplit(sys.argv[1])
METHOD, PATTERN, DELAY = sys.argv[2], re.compile(sys.argv[3]), float(sys.argv[4])
held = threading.Event()


class Proxy(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def forward(self):
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if length else None
        headers = {k: v for k, v in self.headers.items() if k.lower() not in ("host", "connection")}
        upstream = http.client.HTTPConnection(UPSTREAM.hostname, UPSTREAM.port, timeout=60)
        upstream.request(self.command, self.path, body=body, headers=headers)
        reply = upstream.getresponse()
        data = reply.read()
        path = urllib.parse.urlsplit(self.path).path
        print(self.command, path, file=sys.stderr, flush=True)
        if self.command == METHOD and PATTERN.search(path) and not held.is_set():
            held.set()
            time.sleep(DELAY)
        try:
            self.send_response(reply.status)
            for key, value in reply.getheaders():
                if key.lower() not in ("transfer-encoding", "connection", "content-length"):
                    self.send_header(key, value)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        except (BrokenPipeError, ConnectionResetError):
            pass  # the client was killed while the response was held; that is the point

    do_GET = do_POST = do_PATCH = do_PUT = do_DELETE = forward

    def log_message(self, *args):
        pass


server = ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
print(server.server_address[1], flush=True)
server.serve_forever()
