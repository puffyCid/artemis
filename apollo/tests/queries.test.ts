import { describe, expect, it, test } from "vitest";
import { listArtifacts } from "$lib/queries/artifacts";
import { queryCallback } from "$lib/queries/timeline";
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Artifacts } from "$lib/types/search";
import type { TimelineEntry } from "$lib/types/timeline";
import { type State, TableHandler } from "@vincjo/datatables/server";

describe("run query tests", () => {
    test("list artifacts", async () => {
        mockIPC((cmd, _args) => {
            if (cmd === "list_artifacts") {
                return {
                    "_shards": {
                        "failed": 0,
                        "skipped": 0,
                        "successful": 1,
                        "total": 1,
                    },
                    "aggregations": {
                        "artifacts": {
                            "buckets": [{
                                "doc_count": 2944,
                                "key": "FsEvents",
                            }],
                            "doc_count_error_upper_bound": 0,
                            "sum_other_doc_count": 0,
                        },
                    },
                    "hits": {
                        "hits": [],
                        "max_score": null,
                        "total": { "relation": "eq", "value": 2944 },
                    },
                    "timed_out": false,
                    "took": 0,
                };
            }
            return [];
        });
        const result = await listArtifacts() as Artifacts;
        expect(result.aggregations.artifacts.buckets[0].key).toStrictEqual(
            "FsEvents",
        );
    });
    test("query timeline", async () => {
        mockIPC((cmd, _args) => {
            if (cmd === "query_timeline") {
                return {
                    "_shards": {
                        "failed": 0,
                        "skipped": 0,
                        "successful": 1,
                        "total": 1,
                    },
                    "hits": {
                        "hits": [{
                            "_id": "n5CfipMBhie49aQMMJZO",
                            "_index": "test",
                            "_score": null,
                            "_source": {
                                "artifact": "FsEvents",
                                "data_type": "macos:fsevents:entry",
                                "datetime": "1970-01-01T00:00:00.000Z",
                                "event_id": 163140,
                                "flags": [
                                    "Removed",
                                    "IsDirectory",
                                    "Mount",
                                    "Unmount",
                                ],
                                "message": "/Volumes/Preboot",
                                "node": 0,
                                "path": "/Volumes/Preboot",
                                "source":
                                    "/home/android/testing/artemis/core/tests/test_data/macos/fsevents/DLS2/0000000000027d79",
                                "source_accessed": "2024-11-10T04:39:20.000Z",
                                "source_changed": "2024-09-26T01:57:44.000Z",
                                "source_created": "1970-01-01T00:00:00.000Z",
                                "source_modified": "2024-07-25T23:48:13.000Z",
                                "timeline_source":
                                    "/home/android/Projects/artemis/apollo/src-tauri/tests/timelines/no_metadata.jsonl",
                                "timestamp_desc": "Source Created",
                            },
                            "sort": [0],
                        }],
                        "max_score": null,
                        "total": { "relation": "eq", "value": 1 },
                    },
                    "timed_out": false,
                    "took": 1,
                };
            }
            return [];
        });

        function setTotalRows(value: number): void {}

        let entries: TimelineEntry[] = [];
        const table = new TableHandler(entries, { rowsPerPage: 100 });
        let state: State = {
            currentPage: 0,
            rowsPerPage: 0,
            offset: 0,
            setTotalRows,
        };
        const result = await queryCallback(state, table);
        table.load((state: State) => queryCallback(state, table));
        expect(result[0].message).toStrictEqual(
            "/Volumes/Preboot",
        );
    });
});
