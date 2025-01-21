/**
 * Only NTFS 3.1 or higher supported
 * TODO:
 * 
 * 1. Add extension to MftEntry
 * 1.5 Verify size is right?
 * 2. Add limit to cache. 10k directories?
 * 3. Check for recursive parent mfts. Cache should stop that?
 * 4. Add tests
 * 5. Fix clippy
 * 6. Fix Windowŧ string?
 * 7. Remove dupes?
 * 8. Compare against another MFT parser. You should have same number of hits or very close
 */