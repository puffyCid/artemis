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
        expect(JSON.parse(response.body)[ "config" ]).toBe("bG9nX3BhdGggPSAiLi90bXAvYXJ0ZW1pcyIKbG9nX2xldmVsID0gIndhcm5pbmciCgpbc2VydmVyXQp1cmwgPSAiaHR0cDovLzEyNy4wLjAuMSIKcG9ydCA9IDgwMDAKaWdub3JlX3NzbCA9IGZhbHNlCmVucm9sbG1lbnQgPSAiZW5kcG9pbnQvZW5yb2xsIgpjb2xsZWN0aW9ucyA9ICJlbmRwb2ludC9jb2xsZWN0aW9ucyIKY29uZmlnID0gImVuZHBvaW50L2NvbmZpZyIKbG9nZ2luZyA9ICJlbmRwb2ludC9sb2dnaW5nIgp2ZXJzaW9uID0gMQprZXkgPSAibXkga2V5IgoKW2RhZW1vbl0KZW5kcG9pbnRfaWQgPSAiIgpjb2xsZWN0aW9uX3BhdGggPSAiL3Zhci9hcnRlbWlzL2NvbGxlY3Rpb25zIgoKCg==");
    });
});