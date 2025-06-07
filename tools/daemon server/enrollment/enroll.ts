import { Type, Static } from "@sinclair/typebox";
import { FastifyReply, FastifyRequest } from "fastify";
import { v4 as uuidv4 } from 'uuid';

export const Enroll = Type.Object({
    enroll_key: Type.String(),
    endpoint_id: Type.String(),
    info: Type.Object({}),
});

export type EnrollType = Static<typeof Enroll>;

export const EnrollResponse = Type.Object({
    node_key: Type.String(),
    node_invalid: Type.Boolean(),
});

export type EnrollReponseType = Static<typeof EnrollResponse>;

export const BadRequest = Type.Object({
    message: Type.String(),
});

export type BadReqestType = Static<typeof BadRequest>;

export async function enrollEndpoint(request: FastifyRequest<{ Body: EnrollType; }>, reply: FastifyReply) {
    const value = request.body;
    console.log(value.endpoint_id);

    const node_key = uuidv4();
    reply.statusCode = 200;
    reply.send({ node_key, node_invalid: false });
}