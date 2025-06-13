import { Type, Static } from "@sinclair/typebox";
import { FastifyReply, FastifyRequest } from "fastify";
import { pipeline } from "node:stream/promises";
import { mkdir } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { MultipartFile } from "@fastify/multipart";
import { IncomingHttpHeaders } from "node:http2";

export const Collect = Type.Object({
    node_key: Type.String(),
});

export type CollectType = Static<typeof Collect>;

export const CollectResponse = Type.Object({
    collection: Type.String(),
    node_invalid: Type.Boolean(),
});

export type CollectTypeResponse = Static<typeof CollectResponse>;

export async function collectionEndpoint(request: FastifyRequest<{ Body: CollectType; }>, reply: FastifyReply) {
    const toml = "CltvdXRwdXRdCm5hbWUgPSAibGludXhfY29sbGVjdGlvbiIKZGlyZWN0b3J5ID0gIi4vdG1wIgpmb3JtYXQgPSAianNvbmwiCmNvbXByZXNzID0gdHJ1ZQp0aW1lbGluZSA9IGZhbHNlCmVuZHBvaW50X2lkID0gImFiZGMiCmNvbGxlY3Rpb25faWQgPSAxCm91dHB1dCA9ICJhcGkiCnVybCA9ICJodHRwOi8vMTI3LjAuMC4xOjgwMDAvdjEvZW5kcG9pbnQvY29sbGVjdGlvbnMvdXBsb2FkcyIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJwcm9jZXNzZXMiClthcnRpZmFjdHMucHJvY2Vzc2VzXQptZDUgPSBmYWxzZQpzaGExID0gZmFsc2UKc2hhMjU2ID0gZmFsc2UKbWV0YWRhdGEgPSBmYWxzZQoKW1thcnRpZmFjdHNdXQphcnRpZmFjdF9uYW1lID0gInN5c3RlbWluZm8iCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAic2hlbGxfaGlzdG9yeSIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJjaHJvbWl1bS1oaXN0b3J5IgoKW1thcnRpZmFjdHNdXQphcnRpZmFjdF9uYW1lID0gImNocm9taXVtLWRvd25sb2FkcyIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJmaXJlZm94LWhpc3RvcnkiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAiZmlyZWZveC1kb3dubG9hZHMiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAiY3JvbiIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJqb3VybmFsIgpbYXJ0aWZhY3RzLmpvdXJuYWxzXQ==";

    console.log(JSON.stringify(request.headers));

    reply.statusCode = 200;
    reply.send({ collection: toml, node_invalid: false });
}

export async function collectionUploadEndpoint(request: FastifyRequest, reply: FastifyReply) {
    console.log(request.headers);
    if (request.headers[ "x-artemis-collection-complete" ] !== undefined) {
        console.log(`Collection completed at ${request.headers[ "x-artemis-collection-complete" ]}`);
    }
    const data = await request.file();
    if (data === undefined) {
        reply.statusCode = 400;
        return reply.send({ message: "Missing multipart data", node_invalid: false });
    }

    await streamFile(data, request.headers);
    reply.statusCode = 200;
    reply.send({ message: "ok", node_invalid: false });
}

async function streamFile(part: MultipartFile, headers: IncomingHttpHeaders) {
    console.log(`Received filename: ${part.filename}. MIME ${part.mimetype}`);
    const endpoint_id = headers[ "x-artemis-endpoint_id" ];
    const collection_id = headers[ "x-artemis-collection_id" ];

    const collection_path = `./build/tmp/${endpoint_id}/${collection_id}`;
    try {
        await mkdir(collection_path, { recursive: true });
    } catch (err: unknown) {
        if (err instanceof Error)
            console.warn(err.message);
    }

    const encoding = headers[ "content-encoding" ];

    // Filename will either be gzip JSONL files or .log files
    let filename = part.filename;

    // If uploads are JSONL and compressed add `.jsonl.gz` to our filename output
    if (encoding === "gzip" && part.mimetype === "application/jsonl") {
        filename = `${filename}.jsonl.gz`;
    }

    // Output files to endpoint ID and collection ID directories
    await pipeline(part.file, createWriteStream(`${collection_path}/${filename}`));
}