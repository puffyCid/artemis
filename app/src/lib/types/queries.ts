/**
 * Basic interface to make simple queries against OpenSearch
 */
export interface QueryState {
    /**How many rows to return */
    limit: number;
    /**Row offset to start at */
    offset: number;
    /**Order direction */
    order: Ordering;
    /**JSON search query that follows one of the OpenSearch query specs: https://opensearch.org/docs/latest/search-plugins/ */
    query: Record<string, unknown>;
}

export enum Comparison {
    EQUAL = 1,
    LIKE = 0,
}

export enum Ordering {
    ASC = "asc",
    DSC = "desc",
}
