// Downloads the correct `trc` binary from GitHub Releases into ./bin during
// `npm install`. Honors TRACE_VERSION. Fails soft with a clear message so
// installs do not hard-error in CI without network access — and even if this
// is skipped entirely (`--ignore-scripts`), the launcher (bin/trc.js) will
// download the binary on first run, so `trc` still works.
import { ensureBinary } from "./platform.js";

ensureBinary({ log: process.stdout }).catch((e) => {
  process.stderr.write(
    `trc: could not download the binary now (${e.message}).\n` +
      `It will be fetched automatically the first time you run \`trc\`,\n` +
      `or install manually from https://github.com/TaxCollector23/trace/releases\n`
  );
  // Do not fail the whole npm install.
  process.exit(0);
});
