Due to Spotify [removing access to the recommendations endpoint](https://developer.spotify.com/blog/2024-11-27-changes-to-the-web-api) of their WebAPI, this tool is now essentially pointless. Hopefully this change gets reverted, but if not, the development of this tool is ended.

# Spotify-cli

Simple CLI tool for
- creating playlists with Spotify's recommendation engine using the [Web API](https://developer.spotify.com/documentation/web-api/reference/get-recommendations),
- and managing Spotify playback.

This tool will not actually play anything, you need Spotify running somewhere. This tool will only manage already active playback.

**Absolutely no affiliation with Spotify, obviously.**

# What's the point?

I often listen to daily mixes, but sometimes none of them quite match what I'd want to listen to. The [recommendation API](https://developer.spotify.com/documentation/web-api/reference/get-recommendations) lets me generate playlists on the fly to match exactly what I'd like to listen to; this CLI tool let's me use this API.

Also, I almost always listen to Spotify from my phone while working. I wanted to simply be able to control playback and jump between playlists without having to get my phone out while working.

*Why not use something like [rspotify](https://github.com/ramsayleung/rspotify) to handle the Spotify API interactions?* I wrote the auth and API calls from scratch just as an exercise.

If this CLI is actually useful to you and you find you'd like some functionality to be added or something is too bugged for you to use, let me know with an issue or just open a PR.

# Get started

- Clone the repo, build with cargo, and (optional) copy the binary to somewhere in your PATH.
  - If you don't have cargo, get it with [rustup](https://rustup.rs/).
- [Create an app](https://developer.spotify.com/documentation/web-api/concepts/apps) in the Spotify developer dashboard.
- Add `http://localhost:5555` as a redirect URI for the app in the dashboard.
  - This is the default redirect URI the tool tries to use when going through the OAuth flow.
  - If the port is taken, the tool will try 5556, then 5557, ..., up to 5559 before giving up.
  - You need to add `http://localhost:5556`, ..., `http://localhost:5559` as redirect URIs in the dashboard for the app to allow this to work.
  - Or you can just make sure 5555 is available when doing the first auth flow.
- Get the client id and secret for the app from the dashboard and set them as the following environment variables:
  - SPOTIFY_CLI_CLIENT_ID,
  - SPOTIFY_CLI_CLIENT_SECRET.
- You're now set up for playback controls. Run `spotify-cli help` to see the help message and available commands.
- To use recommendations, there are a few more steps:
  - Run `spotify-cli rec init` to create a playlist for the CLI to manage. This will be used to store recommendation lists.
  - The command will print an environment variable you need to set.
  - Done! You can generate recommendations with `spotify-cli rec generate`.
  - See `spotify-cli rec --help` for the commands to play/show/save the generated recommendations.

PS. Only tested with Linux. Might work on Win/Mac, might not.
