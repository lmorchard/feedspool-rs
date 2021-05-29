import { html, createElement, BaseElement } from "./dom.js";

customElements.define(
  "feed-reader",
  class extends BaseElement {
    static template = html`
      <template>
        <style>
          * {
            font-family: sans-serif;
            font-size: 14px;
          }
          :host > header {
            margin: 0.5em 1em;
          }
          .feeds {
            margin: 0.5em;
          }
        </style>
        <section class="feeds"></section>
      </template>
    `;

    propsChanged({ since, feeds }) {
      this.updateElements({
        ".feeds": {
          children: feeds.map((feed) =>
            createElement("feed-reader-feed", { props: feed })
          ),
        },
      });
    }
  }
);

customElements.define(
  "feed-reader-feed",
  class extends BaseElement {
    static template = html`
      <template>
        <style>
          .feed {
            padding: 0.5em;
            margin: 0;
          }
          .feed > header {
            display: block;
            font-size: 1.5em;
            padding: 0.5em 1em;
            margin: 0 0 1em 0;
            background-color: #fff;
            border: 1px solid #aaa;
            box-shadow: 4px 4px 3px rgba(0, 0, 0, 0.4);
          }
          .feed > .entries {
            width: 100%;
            margin: 1em 0.25em;
            display: flex;
            flex-direction: row;
            flex-wrap: wrap;
            justify-content: flex-start;
            align-items: stretch;
            align-content: center;
          }
          .feed > .entries > feed-reader-entry {
            width: 18%;
          }
        </style>
        <section class="feed">
          <header>
            <span class="title">
              <img
                class="feedicon lazy-load"
                width="16"
                height="16"
                data-src=""
              />
              <a href="">Title</a>
            </span>
            <span class="published"></span>
          </header>
          <div class="entries"></div>
        </section>
      </template>
    `;

    propsChanged({ title, link, lastEntryPublished, entries }) {
      let feedHostname;
      try {
        const feedUrl = new URL(link);
        feedHostname = feedUrl.hostname;
      } catch (e) {
        console.log("Bad feed link for", title);
      }

      this.updateElements({
        ".title a": {
          textContent: title,
          "@href": link,
        },
        ".feedicon": {
          src: `https://www.google.com/s2/favicons?domain=${feedHostname}`,
        },
        ".published": {
          textContent: lastEntryPublished,
        },
        ".entries": {
          children: entries.map((entry) =>
            createElement("feed-reader-entry", { props: entry })
          ),
        },
      });
    }
  }
);

customElements.define(
  "feed-reader-entry",
  class extends BaseElement {
    static template = html`
      <template>
        <style>
          :host {
            position: relative;
            padding-bottom: 2em;
            margin-bottom: 1em;
            list-style-type: none;
            background-color: var(--item-bg-color);
            border: 1px solid var(--item-border-color);
            box-shadow: 4px 4px 3px var(--box-shadow-color);

            background-repeat: no-repeat;
            background-size: 100%;

            flex-grow: 1;
            margin-right: 1.25em;

            flex-basis: calc(100% / 5);
            max-width: calc(100vw / 5 - 2em);
          }

          summary {
            position: relative;
          }

          .has-thumb summary {
            min-height: 6em;
          }

          .thumb {
            display: block;
            max-height: 20em;
            overflow: hidden;
          }

          .thumb img {
            max-width: 100%;
          }

          .title {
            width: calc(100% - 2em);
            padding: 2.5em 1em 0em 1em;
            display: block;
            font-weight: 600;
            overflow-wrap: break-word;
            text-decoration: none;
            color: var(--title-color);
            background-color: var(--title-bg-color);
          }

          .feeditem.has-thumb summary .title {
            position: absolute;
            bottom: 0em;
            padding: 0.5em 1em;
            color: var(--title-has-thumb-color);
            background-color: var(--title-has-thumb-bg-color);
            text-shadow: 2px 2px 2px var(--text-shadow-color);
          }

          .text {
            padding: 2em 1em 1em 1em;
            display: block;
            margin-bottom: 0.25em;
            color: var(--item-text-color);
            background-color: var(--item-text-bg-color);
          }

          .feeditem.has-thumb .text {
            padding: 1em 1em 1em 1em;
          }

          .author {
            display: block;
            position: absolute;
            right: 1.25em;
            bottom: 0.75em;
            font-size: 0.85em;
            width: calc(33% - 2em);
            text-align: right;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            color: var(--title-color);
          }

          .date {
            display: block;
            position: absolute;
            left: 1.25em;
            top: 0.75em;
            font-size: 0.8em;
          }

          .date a {
            text-decoration: none;
            color: var(--title-color);
          }

          .has-thumb .date {
            background-color: var(--title-has-thumb-bg-color);
            text-shadow: 2px 2px 2px var(--text-shadow-color);
            left: 0;
            top: 0;
            padding: 0.5em 1.25em;
          }

          .feeditem.has-thumb .date a {
            color: var(--title-has-thumb-color);
          }

          .source {
            display: block;
            position: absolute;
            left: 1.25em;
            bottom: 0.75em;
            width: calc(66% - 2em);
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            font-size: 0.85em;
          }

          .source .link {
            text-decoration: none;
            color: var(--title-color);
          }

          .source .icon {
            margin-right: 0.5em;
            vertical-align: text-bottom;
          }
        </style>

        <summary>
          <!--
            <a target="_blank" class="thumb" href="">
              <img src="" />
            </a>
          -->
          <a class="title" target="_blank" href=""></a>
        </summary>

        <div class="content"></div>

        <div class="date">
          <a class="datelink" target="_blank" href=""></a>
        </div>
      </template>
    `;

    propsChanged({ title, link, content, published }) {
      let feedHostname;
      try {
        const feedUrl = new URL(link);
        feedHostname = feedUrl.hostname;
      } catch (e) {
        console.log("Bad feed link for", title);
      }

      this.updateElements({
        "summary a.title": {
          textContent: title,
          "@href": link,
        },
        ".datelink": {
          "@href": link,
          textContent: published,
        },
        ".content": {
          children: !content
            ? []
            : [
                createElement("iframe", {
                  frameBorder: "0",
                  src: "data:text/html;charset=utf-8," + encodeURI(content),
                  "@style": "height:100%;width:100%;",
                }),
              ],
        },
      });
    }
  }
);
