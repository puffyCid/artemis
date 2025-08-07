import { Type, Static } from "@sinclair/typebox";
import { FastifyRequest, FastifyReply } from "fastify";
import { LocalSqlite } from "../database/db";

export const Logs = Type.Object({
    endpoint_id: Type.String(),
    logs: Type.Array(Type.String()),
});

export type LogsType = Static<typeof Logs>;

export const LogsResponse = Type.Object({
    endpoint_invalid: Type.Boolean(),
});

export type LogsResponseType = Static<typeof LogsResponse>;

/**
 * Handle requests to upload log messages
 * @param request Artemis request containing an array of log messages and the endpoint_id. There is a hard coded limit of 1000 lines
 * @param reply Boolean JSON response to indicator if the endpoint_id is invalid
 */
export async function loggingEndpoint(request: FastifyRequest<{ Body: LogsType; }>, reply: FastifyReply) {
    const value = request.body;
    console.log(`Got log from ${value.endpoint_id}`);
    const db = new LocalSqlite("./build/test.db");
    if (!db.validateEndpoint(request.body.endpoint_id)) {
        reply.statusCode = 500;
        reply.send({ message: `Endpoint not found in database`, endpoint_invalid: true });
        return;
    }

    reply.statusCode = 200;
    reply.send({ endpoint_invalid: false });
}