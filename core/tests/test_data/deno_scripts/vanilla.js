// https://raw.githubusercontent.com/puffycid/artemis-api/master/src/windows/processes.ts
function getWinProcesses(md5, sha1, sha256, pe_info) {
  const hashes = {
    md5,
    sha1,
    sha256,
  };
  const data = js_get_processes(
    hashes,
    pe_info,
  );
  return data;
}

// main.ts
function main() {
  const md5 = false;
  const sha1 = false;
  const sha256 = false;
  const pe_info = false;
  const proc_list = getWinProcesses(md5, sha1, sha256, pe_info);
  console.log(proc_list[0].full_path);
  return proc_list;
}
main();
