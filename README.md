# URL Expander
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/StanleyMasinde/url_expander)

### What is it?

The URL Expander API allows you to expand shortened URLs and return the final destination URL. This helps bypass tracking mechanisms and ensures privacy.
It can also be used as a proxy to bypass CORS.

Bypassing CORS is not a good idea unless you are in development. Or in this case, you might need to bypass CORS so that you can
Preview Links from your front-end applications. This is how [Lnky](https://lnky.stanleymasinde.com), does it. It routes the URL through the `proxy` endpoint.

### Usage

#### Expand a URL
GET lnky.api.stanleymasinde.com?url=<shorturl>
##### Example:
```shell
  curl "https://lnky.api.stanleymasinde.com?url=https://bit.ly"
```

#### Proxy a URL to bypass CORS:
GET lnky.api.stanleymasinde.com/proxy?url=<url>
#### Example:
```shell
  curl "https://lnky.api.stanleymasinde.com/proxy?url=https://stanleymasinde.com"
```

### Response Format

* The API returns a plain text response containing the final URL or plain HTML. It does not return JSON or HTML.

### Is this Deployed?

Yes, it is part of my [Lnky project on GitHub](https://github.com/StanleyMasinde/Lnky). It is responsible for expanding short links like bit.ly.

