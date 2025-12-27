# TreeSheets WebAssembly Deployment Guide

This guide explains how to build and deploy the TreeSheets WebAssembly version.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Building](#building)
- [Web Server Configuration](#web-server-configuration)
- [Security Headers](#security-headers)
- [Browser Requirements](#browser-requirements)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### 1. Emscripten SDK

Install the latest Emscripten SDK:

```bash
# Clone the repository
git clone https://github.com/emscripten-core/emsdk.git
cd emsdk

# Install and activate the latest SDK
./emsdk install latest
./emsdk activate latest

# Activate PATH and other environment variables in the current terminal
source ./emsdk_env.sh
```

### 2. Verify Installation

```bash
emcc --version
# Should output: emcc (Emscripten gcc/clang-like replacement) X.Y.Z
```

---

## Building

### Quick Build

```bash
cd src/wasm
./build.sh release
```

### Build Options

**Release Build** (optimized, smaller file size):
```bash
./build.sh release
```

**Debug Build** (with source maps, assertions):
```bash
./build.sh debug
```

### Build Outputs

After building, you'll find these files in `dist/wasm/`:

- `treesheets.html` - Main HTML file
- `treesheets.js` - JavaScript glue code
- `treesheets.wasm` - WebAssembly binary

---

## Web Server Configuration

### MIME Types

Ensure your web server serves `.wasm` files with the correct MIME type:

```
application/wasm
```

#### Apache (.htaccess)

```apache
AddType application/wasm .wasm
```

#### Nginx (nginx.conf)

```nginx
types {
    application/wasm wasm;
}
```

#### Express.js (Node)

```javascript
const express = require('express');
const app = express();

app.use(express.static('dist/wasm', {
    setHeaders: (res, path) => {
        if (path.endsWith('.wasm')) {
            res.set('Content-Type', 'application/wasm');
        }
    }
}));
```

---

## Security Headers

### Required Headers

For optimal security and compatibility, configure these HTTP headers:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:
X-Content-Type-Options: nosniff
X-Frame-Options: SAMEORIGIN
```

### Why These Headers?

- **COOP/COEP**: Required for SharedArrayBuffer support (future enhancement)
- **CSP**: Prevents XSS attacks while allowing WebAssembly
- **X-Content-Type-Options**: Prevents MIME type sniffing
- **X-Frame-Options**: Prevents clickjacking

### Header Configuration

#### Apache (.htaccess)

```apache
<IfModule mod_headers.c>
    Header set Cross-Origin-Opener-Policy "same-origin"
    Header set Cross-Origin-Embedder-Policy "require-corp"
    Header set Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:"
    Header set X-Content-Type-Options "nosniff"
    Header set X-Frame-Options "SAMEORIGIN"
</IfModule>
```

#### Nginx (nginx.conf)

```nginx
add_header Cross-Origin-Opener-Policy "same-origin";
add_header Cross-Origin-Embedder-Policy "require-corp";
add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:";
add_header X-Content-Type-Options "nosniff";
add_header X-Frame-Options "SAMEORIGIN";
```

---

## HTTPS Requirement

**TreeSheets Web MUST be served over HTTPS** because:

1. **Clipboard API** requires secure context (HTTPS)
2. **Async dialogs** may require HTTPS in some browsers
3. Security best practices for modern web applications

### Local Development

For local testing without HTTPS, use:

```bash
# Python 3
cd dist/wasm
python3 -m http.server 8000

# Or Node.js with http-server
npx http-server dist/wasm -p 8000
```

**Note**: Clipboard API will not work in local HTTP mode.

### Production Deployment

Use Let's Encrypt for free HTTPS certificates:

```bash
# Install Certbot
sudo apt-get install certbot python3-certbot-apache

# Get certificate
sudo certbot --apache -d yourdomain.com
```

---

## Browser Requirements

### Minimum Versions

| Browser | Minimum Version |
|---------|----------------|
| Chrome  | 57+ (Mar 2017) |
| Firefox | 52+ (Mar 2017) |
| Safari  | 11+ (Sep 2017) |
| Edge    | 16+ (Oct 2017) |

### Required Features

- WebAssembly support
- ES6 (arrow functions, const/let, promises)
- Canvas 2D API
- LocalStorage
- FileReader API
- Clipboard API (HTTPS only)

### Browser Detection

The HTML template includes automatic browser compatibility checking. Users on outdated browsers will see a clear error message.

---

## File Size & Performance

### Expected Sizes

**Release Build**:
- treesheets.wasm: ~2-5 MB (depends on code size)
- treesheets.js: ~100-300 KB
- Total initial load: 2-6 MB

### Optimization Tips

1. **Enable Gzip/Brotli compression** on your web server
2. **Set proper cache headers** for .wasm and .js files
3. **Use CDN** for global distribution
4. **Enable HTTP/2** for faster parallel downloads

### Cache Headers Example

```apache
<FilesMatch "\.(wasm|js)$">
    Header set Cache-Control "public, max-age=31536000, immutable"
</FilesMatch>

<FilesMatch "\.html$">
    Header set Cache-Control "no-cache"
</FilesMatch>
```

---

## Deployment Checklist

Before going live, verify:

- [ ] HTTPS is enabled
- [ ] MIME type for `.wasm` is set to `application/wasm`
- [ ] Security headers are configured
- [ ] Gzip/Brotli compression is enabled
- [ ] Cache headers are set
- [ ] Tested in Chrome, Firefox, Safari, Edge
- [ ] Clipboard functionality works (paste/copy)
- [ ] File upload works
- [ ] LocalStorage persistence works
- [ ] All dialogs work correctly (text input, number, color picker)

---

## Troubleshooting

### "ASYNCIFY is not enabled" Error

**Problem**: Build was done without ASYNCIFY flag

**Solution**: Rebuild with `./build.sh release` (ASYNCIFY is included in build script)

### Clipboard Not Working

**Problem**: Page is served over HTTP, not HTTPS

**Solution**: Deploy with HTTPS or test clipboard manually

### "Failed to fetch" Error on .wasm File

**Problem**: CORS headers not configured or wrong MIME type

**Solution**:
1. Check MIME type is `application/wasm`
2. Ensure CORS headers allow wasm loading
3. Check browser console for specific error

### High Memory Usage

**Problem**: Image cache or file cache growing too large

**Solution**: Clear LocalStorage periodically or implement LRU eviction (already included in phase 5 fixes)

### Slow Initial Load

**Problem**: Large WASM file download

**Solution**:
1. Enable Brotli/Gzip compression
2. Use CDN
3. Implement service worker for caching (future enhancement)

---

## Production Deployment Example

### Using Nginx

Complete nginx.conf example:

```nginx
server {
    listen 443 ssl http2;
    server_name treesheets.example.com;

    # SSL configuration
    ssl_certificate /etc/letsencrypt/live/treesheets.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/treesheets.example.com/privkey.pem;

    # Security headers
    add_header Cross-Origin-Opener-Policy "same-origin";
    add_header Cross-Origin-Embedder-Policy "require-corp";
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:";
    add_header X-Content-Type-Options "nosniff";
    add_header X-Frame-Options "SAMEORIGIN";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # MIME types
    types {
        application/wasm wasm;
    }

    # Compression
    gzip on;
    gzip_types application/wasm application/javascript text/html text/css;

    # Cache configuration
    location ~* \.(wasm|js)$ {
        root /var/www/treesheets/dist/wasm;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    location / {
        root /var/www/treesheets/dist/wasm;
        try_files $uri $uri/ /treesheets.html;
        add_header Cache-Control "no-cache";
    }
}
```

---

## Support & Issues

For build or deployment issues:

1. Check browser console for errors
2. Verify Emscripten version is up to date
3. Review build output for warnings
4. Test in multiple browsers

---

## License

Same license as TreeSheets main project.
