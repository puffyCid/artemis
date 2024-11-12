/**
 * Basic interface to make simple queries against the SQLITE timeline table
 */
export interface QueryState {
    /**How many rows to return */
    limit: number;
    /**Row offset to start at */
    offset: number;
    /**Data that should be filtered on*/
    filter: unknown;
    /**Timeline column name to filter on */
    column: ColumnName;
    /**Order direction */
    order: Ordering;
    /**TImeline column to order on */
    order_column: ColumnName;
    /**EQUAL or LIKE comparison */
    comparison: Comparison;
    /**JSON key to filter on for the raw json data column */
    json_key: string;
}

export enum Comparison {
    EQUAL = 1,
    LIKE = 0,
}

export enum Ordering {
    ASC = 1,
    DSC = 0,
}

export enum ColumnName {
    MESSAGE = "Message",
    ARTIFACT = "Artifact",
    DATETIME = "Datetime",
    TIMESTAMP_DESC = "TimestampDesc",
    DATA_TYPE = "DataType",
    TAGS = "Tags",
    NOTES = "Notes",
    DATA = "Data",
}
