import { Type, Static } from "@sinclair/typebox";
import { FastifyReply, FastifyRequest } from "fastify";
import { LocalSqlite } from "../database/db";

export const Config = Type.Object({
    endpoint_id: Type.String(),
});

export type ConfigType = Static<typeof Config>;

export const ConfigResponse = Type.Object({
    config: Type.String(),
    endpoint_invalid: Type.Boolean(),
});

export type ConfigTypeResponse = Static<typeof ConfigResponse>;

/**
 * Handle requests for TOML configs the artemis daemon should use
 * @param request Artemis request containing a endpoint_id obtained from enrollment
 * @param reply Base64 encoded TOML config
 */
export async function configEndpoint(request: FastifyRequest<{ Body: ConfigType; }>, reply: FastifyReply) {
    const db = new LocalSqlite("./build/test.db");
    if (!db.validateEndpoint(request.body.endpoint_id)) {
        reply.statusCode = 500;
        reply.send({ message: `Endpoint not found in database`, endpoint_invalid: true });
        return;
    }
    let toml = "bG9nX3BhdGggPSAiLi90bXAvYXJ0ZW1pcyIKbG9nX2xldmVsID0gIndhcm5pbmciCgpbc2VydmVyXQp1cmwgPSAiaHR0cDovLzEyNy4wLjAuMSIKcG9ydCA9IDgwMDAKaWdub3JlX3NzbCA9IGZhbHNlCmVucm9sbG1lbnQgPSAiZW5kcG9pbnQvZW5yb2xsIgpjb2xsZWN0aW9ucyA9ICJlbmRwb2ludC9jb2xsZWN0aW9ucyIKY29uZmlnID0gImVuZHBvaW50L2NvbmZpZyIKbG9nZ2luZyA9ICJlbmRwb2ludC9sb2dnaW5nIgp2ZXJzaW9uID0gMQprZXkgPSAibXkga2V5IgoKW2RhZW1vbl0KZW5kcG9pbnRfaWQgPSAiIgpjb2xsZWN0aW9uX3BhdGggPSAiL3Zhci9hcnRlbWlzL2NvbGxlY3Rpb25zIgoKCg==";

    // If we are in example container. Use http://daemonserver domain
    if (process.env.LISTEN !== undefined) {
        toml = "bG9nX3BhdGggPSAiLi90bXAvYXJ0ZW1pcyIKbG9nX2xldmVsID0gIndhcm5pbmciCgpbc2VydmVyXQp1cmwgPSAiaHR0cDovL2RhZW1vbnNlcnZlciIKcG9ydCA9IDgwMDAKaWdub3JlX3NzbCA9IGZhbHNlCmVucm9sbG1lbnQgPSAiZW5kcG9pbnQvZW5yb2xsIgpjb2xsZWN0aW9ucyA9ICJlbmRwb2ludC9jb2xsZWN0aW9ucyIKY29uZmlnID0gImVuZHBvaW50L2NvbmZpZyIKbG9nZ2luZyA9ICJlbmRwb2ludC9sb2dnaW5nIgp2ZXJzaW9uID0gMQprZXkgPSAibXkga2V5IgoKW2RhZW1vbl0KZW5kcG9pbnRfaWQgPSAiIgpjb2xsZWN0aW9uX3BhdGggPSAiL3Zhci9hcnRlbWlzL2NvbGxlY3Rpb25zIgoKCg==";
    }
    reply.statusCode = 200;
    reply.send({ config: toml, endpoint_invalid: false });
}