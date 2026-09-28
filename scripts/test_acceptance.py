#!/usr/bin/env python3
"""Model-free regression tests for the opt-in API acceptance probe."""
import json
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from acceptance import check


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def respond(self, body):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        self.respond(json.dumps({'data': [{'id': model} for model in self.server.models]}).encode())

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if body.get('stream'):
            chunk = {'choices': [{'delta': {'content': 'hello'}, 'finish_reason': 'stop'}]}
            end = '' if self.server.broken_stream else 'data: [DONE]\n\n'
            self.respond(('data: ' + json.dumps(chunk) + '\n\n' + end).encode())
            return
        message = {'role': 'assistant', 'content': 'hello'}
        reason = 'stop'
        if body.get('tool_choice'):
            message.update(content=None, reasoning_content='Read the tool.', tool_calls=[{
                'id': 'call-1', 'type': 'function',
                'function': {'name': 'get_code', 'arguments': '{}'}}])
            reason = 'tool_calls'
        elif body['messages'][-1]['role'] == 'tool':
            previous = body['messages'][-2]
            assert previous['reasoning_content'] == 'Read the tool.'
            assert body['messages'][-1]['tool_call_id'] == previous['tool_calls'][0]['id']
            message['content'] = body['messages'][-1]['content']
        self.respond(json.dumps({'choices': [{'message': message, 'finish_reason': reason}]}).encode())


class AcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.server.models = ['deepseek-v4-flash']
        self.server.broken_stream = False
        self.thread = threading.Thread(target=self.server.serve_forever)
        self.thread.start()
        self.base = f'http://127.0.0.1:{self.server.server_port}/v1'

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def test_api_contract(self):
        self.assertEqual(check(self.base, 'deepseek-v4-flash', True)['tool_replay'], 'pass')

    def test_false_alias_rejected(self):
        self.server.models.append('deepseek-v4-pro')
        with self.assertRaisesRegex(AssertionError, 'expected one loaded model'):
            check(self.base, 'deepseek-v4-flash', True)

    def test_missing_stream_termination_rejected(self):
        self.server.broken_stream = True
        with self.assertRaisesRegex(AssertionError, 'SSE termination'):
            check(self.base, 'deepseek-v4-flash')


if __name__ == '__main__':
    unittest.main()
