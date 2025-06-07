import { Type, Static } from "@sinclair/typebox";
import { FastifyReply, FastifyRequest } from "fastify";

export const Collect = Type.Object({
    node_key: Type.String(),
});

export type CollectType = Static<typeof Collect>;

export const CollectResponse = Type.Object({
    collection: Type.String(),
    node_invalid: Type.Boolean(),
});

export type CollectTypeResponse = Static<typeof CollectResponse>;

export async function collectionEndpoint(_request: FastifyRequest<{ Body: CollectType; }>, reply: FastifyReply) {
    const toml = "CltvdXRwdXRdCm5hbWUgPSAibGludXhfY29sbGVjdGlvbiIKZGlyZWN0b3J5ID0gIi4vdG1wIgpmb3JtYXQgPSAianNvbiIKY29tcHJlc3MgPSBmYWxzZQp0aW1lbGluZSA9IGZhbHNlCmVuZHBvaW50X2lkID0gImFiZGMiCmNvbGxlY3Rpb25faWQgPSAxCm91dHB1dCA9ICJsb2NhbCIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJwcm9jZXNzZXMiClthcnRpZmFjdHMucHJvY2Vzc2VzXQptZDUgPSBmYWxzZQpzaGExID0gZmFsc2UKc2hhMjU2ID0gZmFsc2UKbWV0YWRhdGEgPSBmYWxzZQoKW1thcnRpZmFjdHNdXQphcnRpZmFjdF9uYW1lID0gInN5c3RlbWluZm8iCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAic2hlbGxfaGlzdG9yeSIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJjaHJvbWl1bS1oaXN0b3J5IgoKW1thcnRpZmFjdHNdXQphcnRpZmFjdF9uYW1lID0gImNocm9taXVtLWRvd25sb2FkcyIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJmaXJlZm94LWhpc3RvcnkiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAiZmlyZWZveC1kb3dubG9hZHMiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAiY3JvbiI=";

    reply.statusCode = 200;
    reply.send({ collection: toml, node_invalid: false });
}