# Implementation Plan: Add `uuid_record` Field to Collection Metadata

## Overview
Add an optional `uuid_record` field to `collection_metadata` that provides a unique UUID identifier per row/record in JSONL and JSON output formats. Currently, the `uuid` field is generated once per artifact file, but we need a unique identifier for each individual record.

## Current State Analysis

### Current Implementation
- **Location**: `forensics/src/output/formats/jsonl.rs` and `forensics/src/output/formats/json.rs`
- **Current Behavior**: 
  - A single UUID is generated per artifact file (line 26 in `jsonl.rs`)
  - This UUID is reused for all records in the same artifact file
  - The UUID is stored in `collection_metadata.uuid` for each record

### Key Code Locations
1. **JSONL Format** (`forensics/src/output/formats/jsonl.rs`):
   - Line 26: Single UUID generated per file
   - Lines 68-84: `collection_metadata` added to each array entry (same UUID for all)
   - Lines 99-111: `collection_metadata` added to single object

2. **JSON Format** (`forensics/src/output/formats/json.rs`):
   - Line 25: Single UUID generated per file
   - Lines 31-47: `collection_metadata` added to each array entry (same UUID for all)
   - Lines 51-67: `collection_metadata` added to single object

3. **UUID Generation** (`forensics/src/utils/uuid.rs`):
   - `generate_uuid()` function already exists and generates v4 UUIDs

## Implementation Plan

### Phase 1: Core Implementation (JSONL Format)

#### 1.1 Modify `jsonl_format` function
**File**: `forensics/src/output/formats/jsonl.rs`

**Changes**:
- Keep the existing `uuid` field (per-file identifier)
- Add `uuid_record` field that generates a unique UUID for each record
- For array processing (lines 66-89):
  - Generate a new UUID for each entry in the loop
  - Add `uuid_record` to the `collection_metadata` JSON object
- For single object processing (lines 98-111):
  - Generate a UUID for the single record
  - Add `uuid_record` to the `collection_metadata` JSON object
- For empty array case (lines 42-63):
  - No `uuid_record` needed (no records to identify)

**Code Pattern**:
```rust
for entry in entries {
    if entry.is_object() {
        let uuid_record = generate_uuid(); // Generate unique UUID per record
        entry["collection_metadata"] = json![{
            "endpoint_id": output.endpoint_id,
            "uuid": uuid,  // Existing per-file UUID
            "uuid_record": uuid_record,  // New per-record UUID
            "id": output.collection_id,
            // ... rest of metadata
        }];
    }
    // ...
}
```

#### 1.2 Modify `raw_jsonl` function (if needed)
**File**: `forensics/src/output/formats/jsonl.rs`

**Decision**: Since `raw_jsonl` doesn't add `collection_metadata`, no changes needed unless we want to add metadata support to raw output.

### Phase 2: JSON Format Support

#### 2.1 Modify `json_format` function
**File**: `forensics/src/output/formats/json.rs`

**Changes**:
- Apply the same pattern as JSONL format
- For array processing (lines 28-49):
  - Generate a new UUID for each entry
  - Add `uuid_record` to `collection_metadata`
- For single object processing (lines 50-68):
  - Generate a UUID for the single record
  - Add `uuid_record` to `collection_metadata`

### Phase 3: Configuration (Optional)

#### 3.1 Add Optional Configuration Field
**File**: `forensics/src/structs/toml.rs`

**Decision Point**: 
- **Option A**: Always include `uuid_record` (simpler, no config needed)
- **Option B**: Add optional `include_record_uuid: bool` field to `Output` struct

**Recommendation**: **Option A** - Always include it since:
- The user request says "automatically generated at the time of collection"
- It's a small overhead (UUID generation is fast)
- Simpler implementation and no breaking changes to config
- Can be made optional later if needed

**If Option B is chosen**:
- Add `pub include_record_uuid: Option<bool>` to `Output` struct
- Default to `true` if not specified
- Check this flag before adding `uuid_record` field

### Phase 4: Testing

#### 4.1 Update Existing Tests
**Files**: 
- `forensics/src/output/formats/jsonl.rs` (lines 223-311)
- `forensics/src/output/formats/json.rs` (lines 109-160)

**Test Updates**:
- Verify that `uuid_record` is present in output
- Verify that each record has a unique `uuid_record` value
- Verify that `uuid` (per-file) remains the same across records
- Verify that `uuid_record` differs between records

#### 4.2 Add New Test Cases
- Test with array of multiple records
- Test with single object
- Test with empty array (should not have `uuid_record`)
- Test that `uuid_record` values are unique across multiple runs

### Phase 5: Documentation

#### 5.1 Code Comments
- Add comments explaining the difference between `uuid` and `uuid_record`
- Document that `uuid_record` is unique per record

#### 5.2 Example Output Documentation
- Update any example outputs in documentation
- Show example with both `uuid` and `uuid_record` fields

## Implementation Details

### UUID Generation Strategy
- Use existing `generate_uuid()` function from `forensics/src/utils/uuid.rs`
- Generate UUID at the point where `collection_metadata` is being added to each record
- No need for UUID reuse or caching (each record gets a fresh UUID)

### Backward Compatibility
- **Fully backward compatible**: Adding a new field doesn't break existing consumers
- Existing `uuid` field remains unchanged
- New `uuid_record` field is additive

### Performance Considerations
- UUID generation is fast (uses `uuid::Uuid::new_v4()`)
- Minimal performance impact for typical use cases
- For very large collections (millions of records), consider if optimization is needed

## Example Output

### Before (Current)
```json
{
  "collection_metadata": {
    "uuid": "79d42205-775b-4083-9125-a7e742fa72fc",
    "artifact_name": "processes",
    ...
  },
  "data": {...}
}
```

### After (With uuid_record)
```json
{
  "collection_metadata": {
    "uuid": "79d42205-775b-4083-9125-a7e742fa72fc",
    "uuid_record": "550e8400-e29b-41d4-a716-446655440000",
    "artifact_name": "processes",
    ...
  },
  "data": {...}
}
```

## Implementation Checklist

- [ ] Phase 1: Modify `jsonl_format` function in `jsonl.rs`
  - [ ] Add `uuid_record` generation in array processing loop
  - [ ] Add `uuid_record` for single object case
  - [ ] Handle empty array case (no uuid_record needed)
  
- [ ] Phase 2: Modify `json_format` function in `json.rs`
  - [ ] Add `uuid_record` generation in array processing loop
  - [ ] Add `uuid_record` for single object case
  
- [ ] Phase 3: Configuration (if Option B chosen)
  - [ ] Add `include_record_uuid` field to `Output` struct
  - [ ] Add conditional logic based on config
  
- [ ] Phase 4: Testing
  - [ ] Update existing tests to verify `uuid_record` presence
  - [ ] Add test for uniqueness of `uuid_record` values
  - [ ] Add test for consistency of `uuid` (per-file) across records
  - [ ] Test edge cases (empty arrays, single objects, etc.)
  
- [ ] Phase 5: Documentation
  - [ ] Add code comments
  - [ ] Update example outputs if needed

## Files to Modify

1. `forensics/src/output/formats/jsonl.rs` - Main JSONL format implementation
2. `forensics/src/output/formats/json.rs` - JSON format implementation
3. `forensics/src/structs/toml.rs` - (Optional) Add config field if needed

## Estimated Impact

- **Lines of Code**: ~20-30 lines modified/added
- **Breaking Changes**: None (additive feature)
- **Performance Impact**: Minimal (UUID generation is fast)
- **Testing**: Update existing tests + add new test cases

## Notes

- The field name `uuid_record` matches the user's example, but could also be named `record_uuid` - using `uuid_record` as specified
- This feature is specifically for JSONL and JSON formats - CSV format may not need this (to be determined)
- The implementation should be straightforward since the UUID generation infrastructure already exists

## Timeline Processing Considerations

**Important**: The timeline processing (`timeline_data`) is called **before** `collection_metadata` is added to records (see line 36 in `jsonl.rs`). This means:
- Timeline processing won't interfere with `uuid_record` generation
- The `check_meta` function in `timeline/src/artifacts/meta.rs` copies metadata from the first entry to all entries, but this happens during timeline processing, which occurs before metadata is added
- **No changes needed** to timeline processing code for this feature
- If timeline processing is modified in the future to preserve `uuid_record` uniqueness, that would be beneficial but not required for initial implementation
