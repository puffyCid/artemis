import { Type, Static } from "@sinclair/typebox";
import { FastifyReply, FastifyRequest } from "fastify";
import { pipeline } from "node:stream/promises";
import { mkdir } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { MultipartFile } from "@fastify/multipart";
import { IncomingHttpHeaders } from "node:http2";
import { LocalSqlite } from "../database/db";

export const Collect = Type.Object({
    node_key: Type.String(),
});

export type CollectType = Static<typeof Collect>;

export const CollectResponse = Type.Object({
    collection: Type.String(),
    node_invalid: Type.Boolean(),
});

export type CollectTypeResponse = Static<typeof CollectResponse>;

/**
 * Handle requests for TOML collections the artemis daemon should execute
 * @param request Artemis request containing a node_key obtained from enrollment
 * @param reply Base64 encoded TOML collection or an error
 */
export async function collectionEndpoint(request: FastifyRequest<{ Body: CollectType; }>, reply: FastifyReply) {
    try {
        const db = new LocalSqlite("./build/test.db");
        const script = db.getCollections(request.body.node_key);
        if (script === undefined) {
            reply.statusCode = 204;
            reply.send();
            return;
        }
        const toml = Buffer.from(script.script, 'base64').toString().replace("REPLACEME", request.body.node_key);;
        const encoded = Buffer.from(toml).toString('base64');

        db.updateCollection(request.body.node_key, script.collection_id, "Running");

        reply.statusCode = 200;
        reply.send({ collection: encoded, node_invalid: false });

    } catch (err: unknown) {
        if (err instanceof Error) {
            console.warn(`Could not read file ${err}`);
        }
        reply.statusCode = 500;
        reply.send({ message: `Failed to read collection toml file` });
    }
}

/**
 * Function to write collection uploads from the artemis daemon
 * @param request Artemis request uploading the results of the TOML collection. The request will contain metadata in the headers.
 * @param reply 200 OK response if the server sucessfully processes the uploading
 */
export async function collectionUploadEndpoint(request: FastifyRequest, reply: FastifyReply) {
    console.log(request.headers);

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
    } else if (headers[ "x-artemis-collection-complete" ] !== undefined) {
        const db = new LocalSqlite("./build/test.db");
        console.log(`Collection completed at ${headers[ "x-artemis-collection-complete" ]}`);

        db.updateCollection(String(endpoint_id), Number(collection_id), "Complete");
    }

    // Output files to endpoint ID and collection ID directories
    await pipeline(part.file, createWriteStream(`${collection_path}/${filename}`));
}