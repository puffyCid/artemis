import type { Property } from "../search";

/**
 * Index info related to the Metadata index
 */
export interface Metadata {
    aliases: Record<string, unknown>;
    mappings: {
        date_detection: boolean;
        properties: {
            artifact_name: Property;
            complete_time: Property;
            endpoint_id: Property;
            hostname: Property;
            id: Property;
            kernel_version: Property;
            load_performance: {
                avg_fifteen_min: Property;
                avg_five_min: Property;
                avg_one_min: Property;
            };
            os_version: Property;
            platform: Property;
            start_time: Property;
            timeline_source: Property;
            uuid: Property;
        };
    };
    settings: {
        index: {
            replication: {
                type: string;
            };
            number_of_shards: string;
            provided_name: string;
            creation_date: string;
            number_of_replicas: number;
            uuid: string;
            version: {
                created: string;
            };
        };
    };
}
