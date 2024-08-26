/*
 * Steps to parse outlook
 *
 * 1. Parse header -- DONE!
 * 2. Parse pages -- in progress!
 *    2.1 Parse block_offset_descriptor_id next?
 * 4. Parse tables -- ??
 *    - Parse Table Context
 *      - Need to determine the number of rows in the Table Context structure :)
 *        - Folders have 4 components! All have the same node_id_num value!
 *          - NormalFolder - i can parse!
 *          - HierarchyTable - i can parse!
 *          - ContentsTable - i can parse (its a descriptor table)
 *          - FaiContentsTable - i can parse!
 * 5. Parse message store <-- next!!
 *
 * (file)/offset = block btree
 * (item)/descriptor = node btree
 *
 * Working implmetation at https://github.com/Jmcleodfoss/pstreader (MIT LICENSE!)
 *  - run with: java -jar explorer-1.1.2.jar (download from: https://github.com/Jmcleodfoss/pstreader/tree/master/explorer)
 */
