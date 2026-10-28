#!/usr/bin/env python3
"""Small deterministic llama-server stand-in used only by tests."""
import argparse
import json
import os
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

parser = argparse.ArgumentParser(add_help=False)
parser.add_argument("--version", action="store_true")
parser.add_argument("--host", default="127.0.0.1")
parser.add_argument("--port", type=int, default=18080)
parser.add_argument("--model")
parser.add_argument("--alias", default="mock")
args, _ = parser.parse_known_args()
if args.version:
    print("mock-llama-server 0.1")
    raise SystemExit(0)
if not args.model or not os.path.isfile(args.model):
    raise SystemExit("mock backend requires an existing --model file")
API_KEY = os.environ.get("LLAMA_API_KEY", "")


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def authorized(self):
        return self.headers.get("Authorization") == f"Bearer {API_KEY}"

    def send_json(self, value, status=200):
        payload = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self):
        if self.path == "/health":
            self.send_json({"status": "ok"})
        elif self.path == "/v1/models" and self.authorized():
            self.send_json({"object": "list", "data": [{"id": args.alias, "object": "model"}]})
        else:
            self.send_json({"error": {"message": "unauthorized"}}, 401)

    def do_POST(self):
        if self.path != "/v1/chat/completions" or not self.authorized():
            self.send_json({"error": {"message": "unauthorized"}}, 401)
            return
        length = int(self.headers.get("Content-Length", "0"))
        request = json.loads(self.rfile.read(length))
        if not request.get("stream"):
            self.send_json({
                "id": "mock", "object": "chat.completion", "model": args.alias,
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 4, "completion_tokens": 1, "total_tokens": 5},
            })
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        for token in ["local", " ", "private", " ", "AI"]:
            event = {"choices": [{"index": 0, "delta": {"content": token}}]}
            self.wfile.write(f"data: {json.dumps(event)}\n\n".encode())
            self.wfile.flush()
            time.sleep(0.015)
        usage = {"choices": [], "usage": {"prompt_tokens": 4, "completion_tokens": 5, "total_tokens": 9}}
        self.wfile.write(f"data: {json.dumps(usage)}\n\n".encode())
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()


ThreadingHTTPServer((args.host, args.port), Handler).serve_forever()
