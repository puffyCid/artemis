import { describe, expect, test } from '@jest/globals';
import { setupFastify } from '../app';
import { LocalSqlite } from '../database/db';

describe('configuration module', () => {
    test('get endpoint config', async () => {
        const server = await setupFastify();
        const db = new LocalSqlite("./build/test.db");
        const list = db.listEndpoints();
        if (list[ 0 ] === undefined) {
            return;
        }
        const headers = { 'accept': 'application/json', 'content-type': 'application/json' };
        const body = { endpoint_id: list[ 0 ][ "endpoint_id" ] };
        const response = await server.inject({ method: 'POST', 'url': '/v1/endpoint/config', body, headers });
        expect(JSON.parse(response.body)[ "config" ]).toBe("W2RhZW1vbl0KZW5kcG9pbnRfaWQgPSAibXkgaW1wb3J0YW50IGtleSIKY29sbGVjdGlvbl9wYXRoID0gIi92YXIvYXJ0ZW1pcy9jb2xsZWN0aW9ucyIKbG9nX2xldmVsID0gIndhcm4iCg==");
    });
});