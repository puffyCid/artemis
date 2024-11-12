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
    /**Number of artifactas ingested */
    artifacts: number;
    /**Number of JSONL files read */
    files: number;
    /**SQLITE db size */
    db: number;
}
