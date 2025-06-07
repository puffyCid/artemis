import fastify from "fastify";
import { BadReqestType, BadRequest, Enroll, enrollEndpoint, EnrollReponseType, EnrollResponse, EnrollType } from "./enrollment/enroll";
import { Config, configEndpoint, ConfigResponse, ConfigType, ConfigTypeResponse } from "./configuration/config";
import { collectionEndpoint, CollectResponse, CollectType, CollectTypeResponse } from "./collections/collect";

function main() {
    const server = fastify();

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
                reply.send({ message: "Bad enroll request" });
            }
            if (request.body.enroll_key !== "my key") {
                reply.statusCode = 400;
                reply.send({ message: "Bad enrollment key" });
            }
            done();
        },
    }, enrollEndpoint);

    server.post<{ Body: ConfigType, Reply: ConfigTypeResponse | BadReqestType; }>("/v1/endpoint/config", {
        schema: {
            body: Config,
            response: {
                200: ConfigResponse,
                400: BadRequest,
            }
        },
        preValidation: (request, reply, done) => {
            if (request.body.node_key === undefined) {
                reply.statusCode = 400;
                reply.send({ message: "Bad config request" });
            }
            done();
        },
    }, configEndpoint);

    server.post<{ Body: CollectType, Reply: CollectTypeResponse | BadReqestType; }>("/v1/endpoint/collections", {
        schema: {
            body: Config,
            response: {
                200: CollectResponse,
                400: BadRequest,
            }
        },
        preValidation: (request, reply, done) => {
            if (request.body.node_key === undefined) {
                reply.statusCode = 400;
                reply.send({ message: "Bad collection request" });
            }
            done();
        },
    }, collectionEndpoint);
    server.listen({ port: 8000 }, (err, address) => {
        if (err) {
            console.error(err);
            process.exit(1);
        }
        console.log(`Mock server listening at ${address}`);
    });
}

main();