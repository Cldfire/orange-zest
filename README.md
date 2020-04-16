# orange-zest

A library that provides functionality to "zest" SoundCloud for your data (such as your likes, playlists, and comments).

This is WIP and intended to be used for archival purposes and/or automating the process of moving your music away from the SoundCloud platform (because it's a flaming train wreck at this point).

**Please** use it responsibly.

## Obtaining SoundCloud auth credentials

Creating a new `Zester` requires that you provide an OAuth token and a Client ID. Both of these can be obtained by poking around in your browser's devtools while logged in to a SoundCloud account.

### OAuth token

You can find the OAuth token in two places:

1. From cookies
    * Find cookies in your devtools panel
    * Locate the cookie named `oauth_token`; its value is the OAuth token
2. From the headers of an API request
    * Find network traffic in your devtools panel
    * Filter requests for `api-v2`
    * Select a `GET` request and look at the headers
    * Find the `Authorization` header; its value is the OAuth token

### Client ID

* Find network traffic in your devtools panel
* Filter requests for `api-v2`
* Select a request and look at the query parameters
* Locate the parameter named `client_id`; its value is the client ID

There are other ways to obtain a client ID, and it is also possible to do it programmatically. I may investigate such options in the future.

## Storing SoundCloud auth credentials

Avoid placing the credentials in your project code if at all possible. For a simple and effective way of allowing developers (yourself and others) to provide credentials for your code to use, take a look at [dotenv](https://github.com/dotenv-rs/dotenv) (a favorite of mine).
