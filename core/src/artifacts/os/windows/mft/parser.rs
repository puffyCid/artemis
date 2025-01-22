/**
 * Only NTFS 3.1 or higher supported
 * TODO:
 * 
 * 1.5 Verify size is right?
 * 2. Add limit to cache. 10k directories?
 * 3. Check for recursive parent mfts. Cache should stop that?
 *    - Check for recursive attribute list
 * 4. Add tests
 * 5. Fix clippy
 * 7. Remove dupes?
 * 8. Do not include base_extensions (ATTRIBUTE_LIST) entries in the final output. Instead combine them with base_entries
 *    1. Requires that we parse the MFT twice? :/ 
 *    2. First parse MFT and only grab the base_extensions. Cache them.
 *    3. Next parse the MFT and only grab the base_entries.
 *    4. Combine the base_extensions with the base_entry. Via index and sequence matching?
 *    5. Once combined you have all of your attributes
 * 9. Remove panics
 */