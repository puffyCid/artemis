import type { Resources } from "./search";

/**
 * Some about info related to apollo
 */
export interface About {
    /**Apollo version */
    apollo: string;
    /**Rust version */
    rust: string;
    /**Tauri version */
    tauri: string;
    /**Compile timestamp */
    build: string;
    /**OpenSearch resource usage */
    resources: Resources;
}
