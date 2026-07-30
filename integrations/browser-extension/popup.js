// Show local daemon status in the popup.
chrome.runtime.sendMessage({ type: "status" }, (resp) => {
  const el = document.getElementById("status");
  if (resp && resp.port) {
    el.innerHTML = `<span class="ok">●</span> Daemon connected on 127.0.0.1:${resp.port}`;
  } else {
    el.innerHTML =
      '<span class="bad">●</span> Daemon not running. Start it with <code>trg dashboard</code> or <code>trg daemon start</code>.';
  }
});
