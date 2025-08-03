import { describe, expect, test } from '@jest/globals';
import { setupFastify } from '../app';
import { LocalSqlite } from '../database/db';

describe('logging module', () => {
    test('upload simple log', async () => {
        const server = await setupFastify();
        const db = new LocalSqlite("./build/test.db");
        const list = db.listEndpoints();
        if (list[ 0 ] === undefined) {
            return;
        }
        const headers = { 'accept': 'application/json', 'content-type': 'application/json' };
        const body = { endpoint_id: list[ 0 ][ "endpoint_id" ], logs: [ "log line 1", "log line 2" ] };
        const response = await server.inject({ method: 'POST', 'url': '/v1/endpoint/logging', body, headers });
        expect(JSON.parse(response.body)[ "endpoint_invalid" ]).toBe(false);
    });
});