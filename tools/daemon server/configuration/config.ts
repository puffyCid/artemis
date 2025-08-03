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
    const toml = "W2RhZW1vbl0KZW5kcG9pbnRfaWQgPSAibXkgaW1wb3J0YW50IGtleSIKY29sbGVjdGlvbl9wYXRoID0gIi92YXIvYXJ0ZW1pcy9jb2xsZWN0aW9ucyIKbG9nX2xldmVsID0gIndhcm4iCg==";

    reply.statusCode = 200;
    reply.send({ config: toml, endpoint_invalid: false });
}