/*
 * TODO:
 * 1. Read registry to get all DLLs, EXEs, SYS, MUI files associated with eventlogs
 *   - later provide option to have directory containing them
 * 2. Read the file to extract either: MessageTable, MUI, or Wevt_template
 * 3. Parse each one:
 *    - MUI smallest. Points to locale that contains the actual DLL :/
 *    - Wevt_template - template info
 *    - MessageTable - the only one 100% required?
 * 4. Map event log data to message strings
 */
