# URL Expander API

### Overview

The **URL Expander API** resolves shortened or redirected URLs to their final destination.
It’s designed for privacy-focused applications — by expanding URLs server-side, you can strip out tracking redirects and reveal where links actually lead.

The API also includes a lightweight proxy endpoint for previewing links in client-side apps that would otherwise hit **CORS** restrictions.
**Note:** This proxy is intended for **development and controlled environments only**. Avoid using it in production unless you fully trust the upstream content.

Lnky’s front-end uses this mechanism to safely preview links through the `/proxy` endpoint.
See [Lnky](https://lnky.stanleymasinde.com) for reference.

---

### Endpoints

#### 1. Expand a URL

Resolves the final destination of a shortened link.

**GET**

```
https://lnky.api.stanleymasinde.com?url=<short_url>
```

**Example:**

```bash
curl -L "https://lnky.api.stanleymasinde.com?url=https://rb.gy/4wqwzf"
```

---

#### 2. Proxy a URL
> This can be used to bypass CORs. I use it in Lnky to show SEO previews.
> One could also use it to temporarily bypass CORs.

Fetches the raw content from a target URL, allowing front-end previews during development.

**GET**

```
https://lnky.api.stanleymasinde.com/proxy?url=<target_url>
```

**Example:**

```bash
curl -L "https://lnky.api.stanleymasinde.com/proxy?url=https://stanleymasinde.com"
```

---

### Response Format

The API returns:

* **Plain text** containing the final expanded URL, or
* **Raw HTML** when using `/proxy`.

It does **not** return JSON or any structured metadat unless the procied endpoint is structured. 

---

### Deployment

This service is deployed as part of [Lnky](https://github.com/StanleyMasinde/Lnky), a privacy-focused link untracker and expander written in Rust.
It powers Lnky’s backend expansion logic for services like Bitly and t.co.

