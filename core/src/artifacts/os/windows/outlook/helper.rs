/*
 * Steps to parse outlook
 *
 * 1. Parse header -- DONE!
 * 2. Parse pages -- in progress!
 *   - figure out final btree struct for block btree
 *      1. Need to identify how to make them unique :/. The block ID is not unique enough?
 *      2. Maybe parse block_offset_descriptor_id next? Maybe it contains more unique block_offset_data_id?
 * 3. Parse local descriptors (https://github.com/libyal/libpff/blob/main/documentation/Personal%20Folder%20File%20(PFF)%20format.asciidoc#10-the-local-descriptors) -- ??
 * 4. Parse tables -- ??
 *
 * (file)/offset = block btree
 * (item)/descriptor = node btree
 */
