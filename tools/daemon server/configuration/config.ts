import { Type, Static } from "@sinclair/typebox";
import { FastifyReply, FastifyRequest } from "fastify";

export const Config = Type.Object({
    node_key: Type.String(),
});

export type ConfigType = Static<typeof Config>;

export const ConfigResponse = Type.Object({
    config: Type.String(),
    node_invalid: Type.Boolean(),
});

export type ConfigTypeResponse = Static<typeof ConfigResponse>;

/**
 * Handle requests for TOML configs the artemis daemon should use
 * @param request Artemis request containing a node_key obtained from enrollment
 * @param reply Base64 encoded TOML config
 */
export async function configEndpoint(_request: FastifyRequest<{ Body: ConfigType; }>, reply: FastifyReply) {
    const toml = "W2RhZW1vbl0Kbm9kZV9rZXkgPSAibXkgaW1wb3J0YW50IGtleSIKY29sbGVjdGlvbl9zdG9yYWdlID0gIi92YXIvYXJ0ZW1pcy9jb2xsZWN0aW9ucyIKbG9nX2xldmVsID0gIndhcm4iCg==";

    reply.statusCode = 200;
    reply.send({ config: toml, node_invalid: false });
}