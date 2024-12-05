/**
 * Timeline query interface that can used to review data from the SQLITE database
 */
export interface OpenSearchData {
    /**How long OpenSearch took in milliseconds */
    took: number;
    /**If query timed out */
    timed_out: boolean;
    /**Shard info */
    "_shards": {
        total: number;
        successful: number;
        skipped: number;
        failed: number;
    };
    /**Hit info */
    hits: {
        /**Info on total entries */
        total: {
            /**Max number of entries in Index */
            value: number;
            relation: string;
        };
        max_score: unknown;
        /**Array of `Hit` details */
        hits: Hit[];
    };
}

/**
 * Details on OpenSearch search hits
 */
export interface Hit {
    /**Index name */
    "_index": string;
    /**ID name */
    "_id": string;
    "_score": unknown;
    /**Timeline object */
    "_source": TimelineEntry;
    "sort": number[];
}

/**
 * Timeline interface for ingesting data into the SQLITE database.
 * Based on [Timesketch](https://timesketch.org/)
 */
export interface TimelineEntry {
    /** **Required by Timeskech** ISO8601 timestamp format: YYYY-MM-DD HH:mm:ss. All times are in UTC */
    datetime: string;
    /** **Required by Timeskech** Description of the timestamp. Ex: FileCreated */
    timestamp_desc: string;
    /** **Required by Timeskech** Timeline message data */
    message: string;
    /**The type of artifact that was timelined */
    artifact: string;
    /**
     * Artifact data type. Based on plaso definition
     * (its kind of freeform, https://github.com/log2timeline/plaso/blob/main/docs/sources/user/Scribbles-about-events.md).
     * Looks like: `source:artifact:artifact:data`. With first artifact most generic and second one more specific
     * :artifact: can be nested. Ex: `windows:registry:explorer:programcache`
     */
    data_type: string;
    /**Include any other valid JSON data */
    [key: string]: unknown;
}
