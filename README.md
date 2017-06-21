**This website no longer exists, but the code will remain here. Please open an issue if you
have any questions.**

# yaus
**Y**et **A**nother **URL** **S**hortener in Rust.

YAUS was inspired by [this comment on HN](https://news.ycombinator.com/item?id=11957494).
I had wanted to write a web service in Rust and thought it would be a simple exercise
to demonstrate Rust's strengths in speed and safety.

It is implemented using the Iron framework with SQLite for storing the urls. The site has NGINX
sitting in front as a reverse proxy as well.

### Implementation
This app uses Iron's persistent crate to share the SQLite connection between the handlers. The
pooling is provided by r2d2.

To generate the short URL identifier it uses the first seven bytes from the SHA-2 hash of the
original URL. Again, I may have overlooked any issues with this, but the chance of collision
is unlikely.

### API
To shorten a URL you can hit the shorten endpoint:

```sh
curl https://yaus.pw/shorten?url=[1]

[1]: A valid URL
```

Responses:
- 200 The shortened URL exists and has been returned.
- 201 The new shortened URL has been created and returned.
- 400 Invalid request, see the error message.

To retrieve the shortened URL:

```sh
curl https://yaus.pw/[1]

[1]: A valid short identifier
```

Responses:
- 301 The URL has been redirected.
- 404 The identifier has not been found.

### Future Goals
- Share the necessary packages used to build and run the service
- Make the app configurable
- Share NGINX configuration and how to install a certificate with LetsEncrypt
- Add an HTML form to use through a browser
- Add expiration to the links to keep the database from growing too big

### License
MIT
