# feedspool

![Github Actions badge](https://github.com/lmorchard/feedspool-rs/actions/workflows/test.yml/badge.svg)

Feeds on the web need periodic polling. This aims to be a tool for doing that and accumulating the result in a SQLite database for use by other tools.

## To Do

* Port over my thumbnail-scraper to fetch images for feed entries

* OPML import / export

* Feed subscription management

* OPML subscription sync - subscribe to all feeds at an OPML URL, keep local state in sync

* Play with a common library shared between a CLI and a GUI

* Actually document the "public" library API, not just to make clippy happy

* Consider Postgres and MySQL databases?

* Consider expanding beyond RSS & Atom
  * ActivityPub
  * Twitter?
  * HTML scrapers?
