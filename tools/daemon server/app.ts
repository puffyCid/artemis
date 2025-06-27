import { fastify, FastifyInstance } from "fastify";
import fastifyMultipart from "@fastify/multipart";

import { BadReqestType, BadRequest, Enroll, enrollEndpoint, EnrollReponseType, EnrollResponse, EnrollType } from "./enrollment/enroll";
import { Config, configEndpoint, ConfigResponse, ConfigType, ConfigTypeResponse } from "./configuration/config";
import { Collect, collectionEndpoint, collectionUploadEndpoint, CollectResponse, CollectType, CollectTypeResponse } from "./collections/collect";

export async function setupFastify(): Promise<FastifyInstance> {
    const server = fastify();
    server.register(fastifyMultipart, { limits: { fileSize: 314572800 } });

    /** Handle enrollment requests */
    server.post<{ Body: EnrollType, Reply: EnrollReponseType | BadReqestType; }>("/v1/endpoint/enroll", {
        schema: {
            body: Enroll,
            response: {
                200: EnrollResponse,
                400: BadRequest,
            }
        },
        preValidation: (request, reply, done) => {
            if (request.body.enroll_key === undefined) {
                reply.statusCode = 400;
                reply.send({ message: "Bad enroll request", endpoint_invalid: false });
            }
            if (request.body.enroll_key !== "my key") {
                reply.statusCode = 400;
                reply.send({ message: "Bad enrollment key", endpoint_invalid: false });
            }
            done();
        },
    }, enrollEndpoint);

    /** Handle configuration requests */
    server.post<{ Body: ConfigType, Reply: ConfigTypeResponse | BadReqestType; }>("/v1/endpoint/config", {
        schema: {
            body: Config,
            response: {
                200: ConfigResponse,
                400: BadRequest,
            }
        },
        preValidation: (request, reply, done) => {
            if (request.body.endpoint_id === undefined) {
                reply.statusCode = 400;
                reply.send({ message: "Bad config request", endpoint_invalid: false });
            }
            done();
        },
    }, configEndpoint);

    /** Handle collection requests */
    server.post<{ Body: CollectType, Reply: CollectTypeResponse | BadReqestType; }>("/v1/endpoint/collections", {
        schema: {
            body: Collect,
            response: {
                200: CollectResponse,
                400: BadRequest,
            }
        },
        preValidation: (request, reply, done) => {
            if ((request.body as ConfigType).endpoint_id === undefined) {
                reply.statusCode = 400;
                reply.send({ message: "Bad collection request", endpoint_invalid: false });
            }
            done();
        },
    }, collectionEndpoint);

    /** Handle collection uploads */
    server.post("/v1/endpoint/collections/uploads", {
        schema: {
            consumes: [ "multipart/form-data" ],
            response: {
                400: BadRequest,
            }
        },
        preValidation: (request, reply, done) => {
            if (request.headers[ "x-artemis-endpoint_id" ] === undefined || request.headers[ "x-artemis-endpoint_id" ] === "") {
                reply.statusCode = 400;
                reply.send({ message: "Bad upload request. No endpoint ID provided", endpoint_invalid: true });
            }
            done();
        },
    }, collectionUploadEndpoint);

    return server;
}