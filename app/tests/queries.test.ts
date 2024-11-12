import { describe, expect, it, test } from "vitest";
import { listArtifacts } from "../src/lib/queries/artifacts";
import { mockIPC } from "@tauri-apps/api/mocks";

describe("run query tests", () => {
    test("list artifacts", async () => {
        mockIPC((cmd, _args) => {
            if (cmd === "artifacts") {
                return ["fsevents"];
            }
            return [];
        });
        const result = await listArtifacts("");
        expect(result).toStrictEqual(["fsevents"]);
    });
});
