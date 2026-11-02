#!/bin/sh
set -e
cd "$(dirname "$0")"
npx esbuild src/main.js --bundle --minify --format=iife --outfile=out/bundle.js --log-level=warning
python3 - <<'EOF'
html = open('src/page.html').read()
js = open('out/bundle.js').read().replace('</script>', '<\\/script>')
open('out/index.html', 'w').write(html.replace('/*BUNDLE*/', js))
EOF
echo built out/index.html
