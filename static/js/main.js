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

// Increment page view on article detail pages
(function() {
    var el = document.getElementById('page-views');
    if (!el) return;
    var path = window.location.pathname;
    var title = document.querySelector('.post-title');
    var titleText = title ? title.textContent : '';
    fetch('/api/count', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url: path, title: titleText })
    }).then(function(r) { return r.json(); }).then(function(data) {
        if (data.views !== undefined) el.textContent = data.views;
    }).catch(function() { el.textContent = '-'; });
})();

// Load view counts for list pages and render hot list
(function() {
    var countEls = document.querySelectorAll('[data-count-path]');
    var hotList = document.getElementById('hot-list');
    if (!countEls.length && !hotList) return;

    fetch('/api/counts').then(function(r) { return r.json(); }).then(function(data) {
        var countMap = {};
        data.counts.forEach(function(item) { countMap[item.url] = item.views; });

        // Update per-post counts on list pages
        countEls.forEach(function(el) {
            var path = el.getAttribute('data-count-path');
            el.textContent = countMap[path] || 0;
        });

        // Render hot list in sidebar (top 5 from full data)
        if (hotList && data.counts.length > 0) {
            var top10 = data.counts.slice(0, 10);
            fetch('/posts-meta.json').then(function(r) { return r.json(); }).then(function(meta) {
                var titleMap = {};
                meta.forEach(function(item) { titleMap[item.url] = item.title; });
                var html = '';
                top10.forEach(function(item) {
                    var t = titleMap[item.url] || item.title || item.url;
                    html += '<li><a href="' + item.url + '">' + t + '</a> <span class="hot-count">' + item.views + '</span></li>';
                });
                hotList.innerHTML = html;
            }).catch(function() {});
        }
    }).catch(function() {
        countEls.forEach(function(el) { el.textContent = '-'; });
    });
})();
