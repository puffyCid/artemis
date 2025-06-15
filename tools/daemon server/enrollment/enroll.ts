import { Type, Static } from "@sinclair/typebox";
import { FastifyReply, FastifyRequest } from "fastify";
import { v4 as uuidv4 } from 'uuid';
import { LocalSqlite } from "../database/db";

export const Enroll = Type.Object({
    enroll_key: Type.String(),
    endpoint_id: Type.String(),
    info: Type.Object({
        boot_time: Type.String(),
        hostname: Type.String(),
        os_version: Type.String(),
        uptime: Type.Number(),
        kernel_version: Type.String(),
        platform: Type.String(),
        cpu: Type.Array(Type.Object({
            frequency: Type.Number(),
            cpu_usage: Type.Number(),
            name: Type.String(),
            vendor_id: Type.String(),
            brand: Type.String(),
            physical_core_count: Type.Number(),
        })),
        disks: Type.Array(Type.Object({
            disk_type: Type.String(),
            file_system: Type.String(),
            mount_point: Type.String(),
            total_space: Type.Number(),
            available_space: Type.Number(),
            removable: Type.Boolean(),
        })),
        memory: Type.Object({
            available_memory: Type.Number(),
            free_memory: Type.Number(),
            free_swap: Type.Number(),
            total_memory: Type.Number(),
            total_swap: Type.Number(),
            used_memory: Type.Number(),
            used_swap: Type.Number(),
        }),
        performance: Type.Object({
            avg_one_min: Type.Number(),
            avg_five_min: Type.Number(),
            avg_fifteen_min: Type.Number(),
        }),
        interfaces: Type.Array(Type.Object({
            ip: Type.String(),
            name: Type.String(),
            mac: Type.String(),
        })),
        version: Type.String(),
        rust_version: Type.String(),
        build_date: Type.String(),
    }),
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

/**
 * Handle requests for enrollment from artemis daemon. Enrollment uses an enrollment key that is unique for the server
 * @param request Artemis request containing a enrollment key and some system metadata
 * @param reply An assigned node_key for the endpoint
 */
export async function enrollEndpoint(request: FastifyRequest<{ Body: EnrollType; }>, reply: FastifyReply) {
    const value = request.body;

    const node_key = uuidv4();

    const db = new LocalSqlite("./build/test.db");
    db.insertEndpoint(request.body, node_key);

    if (value.info.platform.toLowerCase().includes("linux")) {
        db.newCollection(node_key, 1);
    }

    reply.statusCode = 200;
    reply.send({ node_key, node_invalid: false });
}