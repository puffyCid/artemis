/*
 * Steps to parse outlook
 *
 * 1. Parse header -- DONE!
 * 2. Parse pages -- in progress!
 *    2.1 Parse block_offset_descriptor_id next?
 * 4. Parse tables -- ??
 *    - Parse Table Context
 *      - Need to determine the number of rows in the Table Context structure :)
 *        - You may need to parse all heaps first? -- IN PROGRESS. Get all block_value.data parsing to work perfectly :)
 *        - Need to find the root HID (Heap ID) -- DONE?
 *        - Folders have 4 components! All have the same node_id_num value!
 *          - NormalFolder
 *          - HierarchyTable
 *          - ContentsTable
 *          - FaiContentsTable
 *
 * (file)/offset = block btree
 * (item)/descriptor = node btree
 *
 * Working implmetation at https://github.com/Jmcleodfoss/pstreader (MIT LICENSE!)
 *  - run with: java -jar explorer-1.1.2.jar (download from: https://github.com/Jmcleodfoss/pstreader/tree/master/explorer)
 */
