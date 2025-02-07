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

/**
 * Settings for Apollo to connect to OpenSearch
 */
export interface Settings {
    /**Username for OpenSearch */
    user: string;
    /**Password for OpenSearch */
    creds: string;
    /**Domain name or IP for OpenSearch */
    domain: string;
    /**Current OpenSearch index to use */
    index: string;
    /**Array of OpenSearch indexes */
    indexes: string[];
}
