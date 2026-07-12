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

function resizeMermaidViewport(container, svg) {
    var viewBox = svg.viewBox && svg.viewBox.baseVal;
    if (!viewBox || !viewBox.width || !viewBox.height) return;

    var width = container.clientWidth || svg.clientWidth || viewBox.width;
    var height = Math.round(width * viewBox.height / viewBox.width);
    height = Math.max(280, Math.min(720, height));
    container.style.setProperty('--mermaid-height', height + 'px');
}

function initMermaidZoom() {
    var diagrams = document.querySelectorAll('.mermaid-container .mermaid svg');
    diagrams.forEach(function(svg) {
        if (svg.dataset.zoomReady) return;

        var container = svg.closest('.mermaid-container');
        if (!container) return;

        container.classList.add('mermaid-zoomable');
        resizeMermaidViewport(container, svg);
        svg.setAttribute('width', '100%');
        svg.setAttribute('height', '100%');
        svg.dataset.zoomReady = 'true';

        if (!window.svgPanZoom) return;

        try {
            svg._panZoom = svgPanZoom(svg, {
                controlIconsEnabled: true,
                fit: true,
                center: true,
                minZoom: 0.5,
                maxZoom: 20,
                zoomScaleSensitivity: 0.25
            });
        } catch (e) {
            console.warn('Failed to initialize Mermaid zoom', e);
        }
    });
}

window.initMermaidZoom = initMermaidZoom;

window.addEventListener('resize', function() {
    document.querySelectorAll('.mermaid-container .mermaid svg[data-zoom-ready="true"]').forEach(function(svg) {
        var container = svg.closest('.mermaid-container');
        if (!container) return;
        resizeMermaidViewport(container, svg);
        if (svg._panZoom) {
            svg._panZoom.resize();
            svg._panZoom.fit();
            svg._panZoom.center();
        }
    });
});

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

        // Render hot list in sidebar (top 5 by hot score, only existing posts)
        if (hotList && data.counts.length > 0) {
            fetch('/posts-meta.json').then(function(r) { return r.json(); }).then(function(meta) {
                var existing = {};
                var dateMap = {};
                var titleMap = {};
                meta.forEach(function(item) {
                    existing[item.url] = true;
                    titleMap[item.url] = item.title;
                    dateMap[item.url] = item.date;
                });
                var filtered = data.counts.filter(function(item) { return existing[item.url]; });
                // Hot score: views / (age_in_days + 2)^0.5
                var now = new Date();
                filtered.sort(function(a, b) {
                    var daysA = Math.max(1, (now - new Date(dateMap[a.url])) / 86400000);
                    var daysB = Math.max(1, (now - new Date(dateMap[b.url])) / 86400000);
                    return (b.views / Math.pow(daysB + 2, 0.5)) - (a.views / Math.pow(daysA + 2, 0.5));
                });
                var top5 = filtered.slice(0, 5);
                var html = '';
                top5.forEach(function(item) {
                    var t = titleMap[item.url] || item.url;
                    html += '<li><a href="' + item.url + '">' + t + '</a> <span class="hot-count">' + item.views + '</span></li>';
                });
                hotList.innerHTML = html;
            }).catch(function() {});
        }
    }).catch(function() {
        countEls.forEach(function(el) { el.textContent = '-'; });
    });
})();
