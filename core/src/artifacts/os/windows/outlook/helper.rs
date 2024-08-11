/*
 * Steps to parse outlook
 *
 * 1. Parse header -- DONE!
 * 2. Parse pages -- in progress!
 *    2.1 Parse block_offset_descriptor_id next?
 * 3. Create final struct to return different blocks. Support xblock, raw, and descriptor <--- next!!!
 * 4. Parse tables -- ??
 *
 * (file)/offset = block btree
 * (item)/descriptor = node btree
 *
 * Working implmetation at https://github.com/Jmcleodfoss/pstreader (MIT LICENSE!)
 *  - run with: java -jar explorer-1.1.2.jar (download from: https://github.com/Jmcleodfoss/pstreader/tree/master/explorer)
 */
