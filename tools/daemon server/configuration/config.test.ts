import { describe, expect, test } from '@jest/globals';
import { setupFastify } from '../app';

describe('configuration module', () => {
    test('get endpoint config', async () => {
        const server = await setupFastify();
        const headers = { 'accept': 'application/json', 'content-type': 'application/json' };
        const body = { node_key: "my key" };
        const response = await server.inject({ method: 'POST', 'url': '/v1/endpoint/config', body, headers });
        expect(JSON.parse(response.body)[ "config" ]).toBe("W2RhZW1vbl0Kbm9kZV9rZXkgPSAibXkgaW1wb3J0YW50IGtleSIKY29sbGVjdGlvbl9zdG9yYWdlID0gIi92YXIvYXJ0ZW1pcy9jb2xsZWN0aW9ucyIKbG9nX2xldmVsID0gIndhcm4iCg==");
    });
});