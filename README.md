# Spotify-cli (heavily WIP)

Simple CLI tool for
- managing Spotify playback,
- and creating playlists with Spotify's recommendation engine using the [Web API](https://developer.spotify.com/documentation/web-api/reference/get-recommendations). <- TODO

This tool will not actually play anything, you need Spotify running somewhere. This tool will only manage already active playback.

**Absolutely no affiliation with Spotify, obviously.**

# What's the point?

I almost always listen to Spotify from my phone while working. I wanted to simply be able to control playback and jump between playlists without having to get my phone out while working.

Also, I often listen to daily mixes, but sometimes none of them quite match what I'd want to listen to. The [recommendation API](https://developer.spotify.com/documentation/web-api/reference/get-recommendations) lets me generate playlists on the fly to match exactly what I'd like to listen to; this CLI tool let's me use this API.

*Why not use [rspotify](https://github.com/ramsayleung/rspotify) to handle the Spotify API interactions?* I wrote the auth and API calls from scratch just as an exercise.

If this CLI is actually useful to you and you find you'd like some functionality to be added or something is too bugged for you to use, let me know with an issue or just open a PR.

# Get started

- Clone the repo and build with cargo.
- [Create an app](https://developer.spotify.com/documentation/web-api/concepts/apps) in the Spotify developer dashboard.
- Add `http://localhost:5555` as a redirect URI for the app.
  - This is the default redirect URI the tool tries to use when going through the OAuth flow.
  - If the port is taken, the tool will try 5556, then 5557, ..., up to 5559 before giving up.
  - You can add `http://localhost:5556`, ..., `http://localhost:5559` as redirect URIs for the app to allow this to work.
  - Or you can just make sure 5555 is available when doing the first auth flow.
- Get the client id and secret for the app and set them as the following environment variables:
  - SPOTIFY_CLI_CLIENT_ID,
  - SPOTIFY_CLI_CLIENT_SECRET.
- You're good to go. Run `spotify-cli help` to see the help message and available commands.

PS. Only tested with Linux. Might work on Win/Mac, might not.
