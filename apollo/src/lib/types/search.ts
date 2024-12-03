/**
 * Error status object from OpenSearch
 */
export interface ErrorStatus extends Cause {
    error: {
        root_cause: Cause[];
    };
    status: number;
}

interface Cause {
    type: string;
    reason: string;
    index: string;
    resource_id: string;
    resource_type: string;
    index_uuid: string;
}

/**
 * Info object related to Indices
 */
export interface IndexInfo {
    aliases: Record<string, unknown>;
    mappings: {
        date_detection: boolean;
        /**Property name (key) and Property data or nested properties */
        properties: Record<string, Property | unknown>;
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

/**
 * Property info associated with an `IndexInfo`
 */
export interface Property {
    type: string;
    fields?: {
        keyword: {
            type: "string";
            ignore_above: number;
        };
    };
}

/**
 * Some basic info about OpenSerch resource usage
 * Obtained via: `GET _nodes/stats`
 */
export interface Resources {
    cluster_name: string;
    nodes: Record<string, Nodes>;
}

/**
 * There is alot of info exposed. We only define some of it
 */
interface Nodes {
    name: string;
    ip: string;
    os: {
        cpu: {
            percent: number;
        };
        mem: {
            total_bytes: number;
            used_in_bytes: number;
            free_percent: number;
            used_percent: number;
        };
    };
    process: {
        timestamp: number;
        mem: {
            total_virtual_in_bytes: number;
        };
    };
    jvm: {
        uptime_in_millis: number;
        threads: {
            count: number;
            peak_count: number;
        };
    };
}

/**
 * Info count on number of artifacts ingested
 */
export interface Artifacts {
    "_shards": {
        failed: number;
        skipped: number;
        successful: number;
        total: number;
    };
    aggregations: {
        /**Array of artifacts ingested */
        artifacts: {
            buckets: {
                /**Artifact count */
                doc_count: number;
                /**Artifact name */
                key: string;
            }[];
            doc_count_error_upper_bound: number;
            sum_other_doc_count: number;
        };
    };
    /**Total hit count of artifacts */
    hits: {
        hits: unknown[];
        max_score: null;
        total: {
            relation: string;
            value: number;
        };
    };
    timed_out: boolean;
    took: number;
}
