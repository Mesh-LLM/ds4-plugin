#!/usr/bin/env python3
"""Opt-in API acceptance against an explicitly supplied, already running endpoint.

Never starts/stops Mesh, loads weights, changes configuration or downloads files.
The caller owns the endpoint and must authorize inference before invoking this.
"""
import argparse
import json
import urllib.request


def request(base, path, body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(base.rstrip('/') + path, data,
                                 {'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=180) as response:
        return response.read()


def check(base, model, strict_discovery=False):
    models = json.loads(request(base, '/models'))
    ids = [item['id'] for item in models['data']]
    assert model in ids, f'{model} absent from discovery: {ids}'
    assert len(ids) == len(set(ids)), f'duplicate model IDs: {ids}'
    if strict_discovery:
        assert ids == [model], f'expected one loaded model, got {ids}'

    def chat(messages, **extra):
        return json.loads(request(base, '/chat/completions', dict(
            model=model, messages=messages, temperature=0, max_tokens=512, **extra)))

    plain = chat([{'role': 'user', 'content': 'Reply with a short greeting.'}])
    assert plain['choices'][0]['message']['content'], 'empty chat content'
    tools = [{'type': 'function', 'function': {
        'name': 'get_code', 'description': 'Read the secret code.',
        'parameters': {'type': 'object', 'properties': {}, 'required': []}}}]
    messages = [{'role': 'user', 'content':
                 'Call get_code and then report the exact code returned by the tool.'}]
    first = chat(messages, tools=tools, tool_choice={
        'type': 'function', 'function': {'name': 'get_code'}})
    assert first['choices'][0]['finish_reason'] == 'tool_calls'
    assistant = first['choices'][0]['message']
    calls = assistant['tool_calls']
    assert len(calls) == 1
    call = calls[0]
    assert call['id'] and call['function']['name'] == 'get_code'
    assert isinstance(json.loads(call['function']['arguments']), dict)
    # Preserve the entire assistant message, including reasoning and tool IDs.
    messages += [assistant, {'role': 'tool', 'tool_call_id': call['id'],
                             'content': 'CODE-7391'}]
    replay = chat(messages, tools=tools)
    assert 'CODE-7391' in replay['choices'][0]['message']['content']
    raw = request(base, '/chat/completions', dict(
        model=model, messages=[{'role': 'user', 'content': 'Say hello briefly.'}],
        stream=True, max_tokens=512, temperature=0)).decode()
    events = [line[6:] for line in raw.splitlines() if line.startswith('data: ')]
    assert events and events[-1] == '[DONE]', 'missing final SSE termination'
    chunks = [json.loads(event) for event in events[:-1]]
    choices = [choice for chunk in chunks for choice in chunk.get('choices', [])]
    assert any(choice.get('finish_reason') for choice in choices), 'missing finish reason'
    assert ''.join(choice.get('delta', {}).get('content') or '' for choice in choices)
    return {'models': ids, 'chat': 'pass', 'tool_replay': 'pass', 'stream': 'pass'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-url', required=True, help='Explicit Mesh or ds4 URL ending in /v1')
    parser.add_argument('--model', default='deepseek-v4-flash')
    parser.add_argument('--strict-discovery', action='store_true',
                        help='Require exactly the selected model on an isolated test host')
    args = parser.parse_args()
    print(json.dumps(check(args.base_url, args.model, args.strict_discovery), indent=2))


if __name__ == '__main__':
    main()
