import { Type, Static } from "@sinclair/typebox";
import { FastifyReply, FastifyRequest } from "fastify";
import { pipeline } from "node:stream/promises";
import { mkdir } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { MultipartFile } from "@fastify/multipart";
import { IncomingHttpHeaders } from "node:http2";
import { LocalSqlite } from "../database/db";

export const Collect = Type.Object({
    endpoint_id: Type.String(),
});

export type CollectType = Static<typeof Collect>;

export const CollectResponse = Type.Object({
    collection: Type.String(),
    endpoint_invalid: Type.Boolean(),
    collection_id: Type.Number(),
});

export type CollectTypeResponse = Static<typeof CollectResponse>;

export const NewCollection = Type.Object({
    endpoint_id: Type.String(),
    collection_id: Type.Number(),
    payload: Type.String(),
});

export type NewCollectionType = Static<typeof NewCollection>;

export const NewCollectionResponse = Type.Object({
    endpoint_invalid: Type.Boolean(),
});

export type NewCollectionResponseType = Static<typeof NewCollectionResponse>;



/**
 * Handle requests for TOML collections the artemis daemon should execute
 * @param request Artemis request containing a endpoint_id obtained from enrollment
 * @param reply Base64 encoded TOML collection or an error
 */
export async function collectionEndpoint(request: FastifyRequest<{ Body: CollectType; }>, reply: FastifyReply) {
    try {
        const db = new LocalSqlite("./build/test.db");
        if (!db.validateEndpoint(request.body.endpoint_id)) {
            reply.statusCode = 500;
            reply.send({ message: `Endpoint not found in database`, endpoint_invalid: true });
        }
        const script = db.getCollections(request.body.endpoint_id);
        if (script === undefined) {
            reply.statusCode = 204;
            reply.send();
            return;
        }
        const toml = Buffer.from(script.script, 'base64').toString().replace("REPLACEME", request.body.endpoint_id);;
        const encoded = Buffer.from(toml).toString('base64');

        db.updateCollection(request.body.endpoint_id, script.collection_id, "Running");

        reply.statusCode = 200;
        reply.send({ collection: encoded, endpoint_invalid: false, collection_id: script.collection_id });

    } catch (err: unknown) {
        if (err instanceof Error) {
            console.warn(`Could not read file ${err}`);
        }
        reply.statusCode = 500;
        reply.send({ message: `Failed to read collection toml file`, endpoint_invalid: false });
    }
}

/**
 * Handle user base64 TOML uploads to collect data from artemis
 * @param request User request to upload a base64 TOML collection 
 * @param reply Upload success or and error
 */
export async function createNewCollection(request: FastifyRequest<{ Body: NewCollectionType; }>, reply: FastifyReply) {
    try {
        const db = new LocalSqlite("./build/test.db");
        if (!db.validateEndpoint(request.body.endpoint_id)) {
            reply.statusCode = 500;
            reply.send({ message: `Endpoint not found in database`, endpoint_invalid: true });
        }

        db.newCollection(request.body.endpoint_id, request.body.collection_id);
        db.newCollectionScript(request.body.payload, request.body.collection_id);

        reply.statusCode = 200;
        reply.send({ endpoint_invalid: false });
    } catch (err: unknown) {
        if (err instanceof Error) {
            console.warn(`Could add new collection ${err}`);
        }
        console.log("wrong");
        reply.statusCode = 500;
        reply.send({ message: `Failed upload new collection`, endpoint_invalid: false });
    }
}

/**
 * Function to write collection uploads from the artemis daemon
 * @param request Artemis request uploading the results of the TOML collection. The request will contain metadata in the headers.
 * @param reply 200 OK response if the server successfully processes the uploading
 */
export async function collectionUploadEndpoint(request: FastifyRequest, reply: FastifyReply) {
    console.log(request.headers);

    const data = await request.file();
    if (data === undefined) {
        reply.statusCode = 400;
        return reply.send({ message: "Missing multipart data", endpoint_invalid: false });
    }
    const db = new LocalSqlite("./build/test.db");
    if (!db.validateEndpoint(request.headers[ "x-artemis-endpoint_id" ] as string)) {
        reply.send({ message: `Endpoint not found in database`, endpoint_invalid: true });
    }

    await streamFile(data, request.headers);
    reply.statusCode = 200;
    reply.send({ message: "ok", endpoint_invalid: false });
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

export const CollectStatus = Type.Object({
    collection_status: Type.String(),
    collection_id: Type.Number(),
});

export type CollectStatusType = Static<typeof CollectStatus>;

export const CollectStatusResponse = Type.Object({
    endpoint_invalid: Type.Boolean(),
});

export type CollectStatusTypeResponse = Static<typeof CollectStatusResponse>;

/**
 * Handle collection status requests. When artemis finishes a collection, the daemon will send a status requests signifying if the collection completed or had an error
 * @param request Artemis request setting the status of a collection
 * @param reply 200 OK response if we can process the collection status
 */
export async function collectionUploadStatusEndpoint(request: FastifyRequest<{ Body: CollectStatusType; }>, reply: FastifyReply) {
    const db = new LocalSqlite("./build/test.db");
    const headers = request.headers;

    const endpoint_id = headers[ "x-artemis-endpoint_id" ];
    if (typeof endpoint_id !== 'string') {
        reply.statusCode = 400;
        return reply.send({ message: "Missing endpoint id", endpoint_invalid: true });
    }
    const collection_id = request.body.collection_id;

    console.log(`Collection status ${request.body.collection_status}`);

    db.updateCollection(endpoint_id, collection_id, request.body.collection_status);
    reply.statusCode = 200;
    return reply.send({ endpoint_invalid: false });

}