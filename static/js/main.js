function copyCode(btn) {
    var block = btn.closest('.code-block');
    var code = block.querySelector('pre.highlight code');
    var clone = code.cloneNode(true);
    clone.querySelectorAll('.ln').forEach(function(ln) { ln.remove(); });
    var text = clone.textContent || clone.innerText;
    navigator.clipboard.writeText(text).then(function() {
        btn.textContent = '已复制';
        btn.classList.add('copied');
        setTimeout(function() {
            btn.textContent = '复制';
            btn.classList.remove('copied');
        }, 2000);
    });
}

function toggleWrap(btn) {
    var block = btn.closest('.code-block');
    block.classList.toggle('wrap');
    btn.classList.toggle('active');
}

// Busuanzi page views for post lists (homepage, tag, category pages)
(function() {
    var items = document.querySelectorAll('.busuanzi-page-views');
    if (!items.length) return;

    var paths = [];
    items.forEach(function(el) {
        paths.push(el.getAttribute('data-path'));
    });

    // Use busuanzi JSONP-like API for each path
    items.forEach(function(el) {
        var path = el.getAttribute('data-path');
        var countEl = el.querySelector('.busuanzi-page-count');
        var callbackName = 'bz_cb_' + path.replace(/[^a-zA-Z0-9]/g, '_');
        window[callbackName] = function(data) {
            if (data && data.page_pv !== undefined) {
                countEl.textContent = data.page_pv;
            } else {
                countEl.textContent = '0';
            }
            delete window[callbackName];
        };
        var script = document.createElement('script');
        script.src = 'https://busuanzi.ibruce.info/busuanzi?jsoncallback=' + callbackName + '&VN=sitedata&UV=siteuv&PG=pagepv&URL=' + encodeURIComponent(path);
        document.body.appendChild(script);
    });
})();
