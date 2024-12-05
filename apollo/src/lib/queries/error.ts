import type { ErrorStatus } from "$lib/types/search";

/**
 * Check if OpenSearch returned an error
 * @param data OpenSearch response
 * @returns Validation if OpenSearch response is `ErrorStatus`
 */
export function isError(data: any): data is ErrorStatus {
    return ((data as ErrorStatus).error != undefined &&
        (data as ErrorStatus).error.root_cause != undefined);
}
