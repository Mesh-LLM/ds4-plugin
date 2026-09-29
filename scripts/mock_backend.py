#!/usr/bin/env python3
"""Fake ds4 executable for released-host integration; no model is loaded."""
import argparse
import os
from pathlib import Path
from http.server import ThreadingHTTPServer
from test_acceptance import Handler

parser = argparse.ArgumentParser()
parser.add_argument('-m')
parser.add_argument('--ctx')
parser.add_argument('--host')
parser.add_argument('--port', type=int)
args = parser.parse_args()
Path('backend.pid').write_text(str(os.getpid()))
server = ThreadingHTTPServer((args.host, args.port), Handler)
server.models = ['deepseek-v4-flash', 'deepseek-v4-pro']
server.broken_stream = False
server.serve_forever()
