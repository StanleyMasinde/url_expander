
# URL Expander

### What is it?

The URL Expander API allows you to expand shortened URLs and return the final destination URL. This helps bypass tracking mechanisms and ensures privacy.

### Usage

* **Expand a URL:**

  ```
  GET lnky.api.stanleymasinde.com?url=<shorturl>
  ```

  * Example: `lnky.api.stanleymasinde.com?url=https://bit.ly`

* **Proxy a URL to bypass CORS:**

  ```
  GET lnky.api.stanleymasinde.com/proxy?url=<url>
  ```

  * Example: `lnky.api.stanleymasinde.com/proxy?url=https://stanleymasinde.com`

### Response Format

* The API returns a plain text response containing the final URL. It does not return JSON.

### ⚠️ Warning

This project is a learning exercise in Rust and is not production-ready. Do not deploy it as-is without thorough optimization and security improvements.

* This could be written in Node.js in less than 50 lines for a simpler implementation.

### Is this Deployed?

Yes, it is part of my [Lnky project on GitHub](https://github.com/StanleyMasinde/Lnky). It is responsible for expanding short links like bit.ly.

### Last Updated

2025-05-16 14:12 UTC+3
