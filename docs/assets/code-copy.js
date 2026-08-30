// Copy-to-clipboard for install command snippets, shared by index.html
// and docs.html. Each opted-in .code-block carries the exact text to
// copy in data-copy (line breaks written as literal "\n", quotes/
// ampersands HTML-escaped — it's a plain attribute, not markup) so the
// button copies precisely what's shown regardless of how the block
// wraps or is styled.
(function () {
  var COPIED_LABEL_MS = 1800;

  function buildButton() {
    var btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'code-copy-btn';
    btn.setAttribute('aria-label', 'Copy to clipboard');
    btn.innerHTML =
      '<svg class="icon-copy" viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="12" height="12" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>' +
      '<svg class="icon-check" viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"></polyline></svg>';
    return btn;
  }

  function flashCopied(btn) {
    btn.classList.add('copied');
    btn.setAttribute('aria-label', 'Copied!');
    window.clearTimeout(btn._copyTimer);
    btn._copyTimer = window.setTimeout(function () {
      btn.classList.remove('copied');
      btn.setAttribute('aria-label', 'Copy to clipboard');
    }, COPIED_LABEL_MS);
  }

  // Fallback for browsers without the async Clipboard API — and for a
  // Clipboard API call that exists but rejects (e.g. permission denied
  // in an insecure or embedded context).
  function legacyCopy(text) {
    var textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);
    textarea.select();
    try {
      document.execCommand('copy');
    } finally {
      textarea.remove();
    }
    return Promise.resolve();
  }

  function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text).catch(function () {
        return legacyCopy(text);
      });
    }
    return legacyCopy(text);
  }

  document.querySelectorAll('.code-block[data-copy]').forEach(function (block) {
    var btn = buildButton();
    block.classList.add('has-copy');
    block.appendChild(btn);
    btn.addEventListener('click', function () {
      var text = block.dataset.copy.replaceAll('\\n', '\n');
      copyText(text)
        .then(function () { flashCopied(btn); })
        .catch(function () {
          // Both the Clipboard API and the execCommand fallback failed
          // (e.g. clipboard permission denied with no execCommand
          // support) — nothing more we can do here.
        });
    });
  });
})();
